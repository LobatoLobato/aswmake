use std::{
    fs::File,
    io::{Cursor, Read, Seek, SeekFrom, Write}
};
use byteorder::{ReadBytesExt, WriteBytesExt, LE};
use crate::{AResult, error, path::Path};

pub fn extract_bytes(uexp_bytes: &Vec<u8>) -> AResult<Vec<u8>> {
    let contained_file = &uexp_bytes[UEXP_FILE_START..uexp_bytes.len() - UEXP_FILE_END_PAD];
    Ok(contained_file.to_vec())
}

pub fn extract(uexp_path: impl Path, out_path: impl Path) -> AResult<()> {
    let out_path = out_path.absolute().or(Err(error::InvalidFilePath(out_path)))?;
    let uexp_path = uexp_path.absolute_file().or(Err(error::InvalidFilePath(uexp_path)))?;
    let mut file = File::create(out_path)?;
    let mut uexp = File::open(uexp_path)?;
    let mut uexp_bytes = Vec::new();

    uexp.read_to_end(&mut uexp_bytes)?;
    
    file.write_all(&extract_bytes(&uexp_bytes)?)?;

    Ok(())
}

/// offset of the 2 values that both hold the size of the contained file
const UEXP_SIZE_OFFSET: usize = 0x24;
const UEXP_FILE_START: usize = 0x34;
const UEXP_FILE_END_PAD: usize = 0x4;
const MYSTERIOUS_NUMBER: usize = 0xA9;

pub fn inject_bytes(
    inject_input_bytes: &Vec<u8>, 
    uexp_bytes: &mut Vec<u8>, 
    uasset_bytes: &mut Vec<u8>
) -> AResult<String> {
    let mut uexp = Cursor::new(uexp_bytes);
    let mut uasset = Cursor::new(uasset_bytes);
    let mut info = vec![];
    
    // Gather data for later
    uexp.seek(SeekFrom::End(-(UEXP_FILE_END_PAD as i64)))?;
    let magic = uexp.read_u32::<LE>()?;

    info.push(format!("Got magic `{:#X}`", magic));

    let total_uasset_size = uasset.get_ref().len() as u32;
    let total_uexp_size = (uexp.get_ref().len() - UEXP_FILE_END_PAD) as u32;
    let total_combined_size = total_uasset_size + total_uexp_size;
    let contained_file_size = (uexp.get_ref().len() - UEXP_FILE_END_PAD) as u32;

    // resize to new needed size
    uexp.get_mut().resize(UEXP_FILE_START + inject_input_bytes.len() + UEXP_FILE_END_PAD, 0);

    uexp.set_position(UEXP_FILE_START as u64);
    uexp.write_all(&inject_input_bytes)?;
    uexp.write_u32::<LE>(magic)?;

    let size_offset = find_seq(uexp.get_ref(), &contained_file_size.to_le_bytes())
        .unwrap_or(UEXP_SIZE_OFFSET);
    
    info.push(format!("Found size offset {:#X}", size_offset));

    uexp.set_position(size_offset as u64);
    let file_bytes_size = inject_input_bytes.len() as u32;
    uexp.write_u32::<LE>(file_bytes_size)?;
    uexp.write_u32::<LE>(file_bytes_size)?;

    let uasset_total_pos = find_seq(uasset.get_ref(), &total_combined_size.to_le_bytes())
        .unwrap_or(MYSTERIOUS_NUMBER);
    let uasset_uexp_pos = match find_seq(uasset.get_ref(), &total_uexp_size.to_le_bytes()) {
        Some(n) => n,
        None => return Err(anyhow::anyhow!("Could not find uassets uexp size description offset")),
    };

    // write uasset + uexp size
    let new_total_size = (uexp.get_ref().len() + uasset.get_ref().len() - UEXP_FILE_END_PAD) as u32;
    info.push(format!("new total uasset + uexp size: {:#X}", new_total_size));
    uasset.set_position(uasset_total_pos as u64);
    uasset.write_u32::<LE>(new_total_size)?;

    // write uexp size
    let new_uexp_size = (uexp.get_ref().len() - UEXP_FILE_END_PAD) as u32;
    info.push(format!("new uexp size: {:#X}", new_uexp_size));
    uasset.set_position(uasset_uexp_pos as u64);
    uasset.write_u32::<LE>(new_uexp_size)?;
    
    Ok(info.join("\n"))
}
pub fn inject(inject_input: impl Path, uexp_path: impl Path, uasset_path: impl Path) -> AResult<String> {
    let inject_input = inject_input.absolute_file().or(Err(error::InvalidFilePath(inject_input)))?;
    let uexp_path = uexp_path.absolute_file().or(Err(error::InvalidFilePath(uexp_path)))?;
    let uasset_path = uasset_path.absolute_file().or(Err(error::InvalidFilePath(uasset_path)))?;

    let mut file = File::open(inject_input)?;
    let mut uexp = File::open(&uexp_path)?;
    let mut uasset = File::open(&uasset_path)?;

    let mut file_bytes = Vec::new();
    let mut uexp_bytes = Vec::new();
    let mut uasset_bytes = Vec::new();

    file.read_to_end(&mut file_bytes)?;
    uexp.read_to_end(&mut uexp_bytes)?;
    uasset.read_to_end(&mut uasset_bytes)?;
    
    let info = inject_bytes(&file_bytes, &mut uexp_bytes, &mut uasset_bytes)?;
    
    let mut new_uasset = File::create(uasset_path)?;
    new_uasset.write_all(&uasset_bytes)?;

    let mut new_uexp = File::create(uexp_path)?;
    new_uexp.write_all(&uexp_bytes)?;
    Ok(info)
}

fn find_seq(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|window| window == needle)
}


#[cfg(test)]
use suitest::{suite, suite_cfg};

#[cfg(test)]
#[suite(tools_bbspack_rs)]
#[suite_cfg(sequential = true, verbose = false)]
mod tests {
    use std::{path::PathBuf, sync::Arc};
    use suitest::before_all;
    use crate::{AResult, util::{Sha1Hash, sha1_hash_file, sha1_hash_bytes}};
    
    #[derive(Debug)]
    struct Context {
        _fixtures_dir: tempfile::TempDir,
        fixtures_dir_path: PathBuf,
        bbscript_ref: PathBuf,
        inject_a_assets: (PathBuf, PathBuf),
        inject_a_hashes: (AResult<Sha1Hash>, AResult<Sha1Hash>),
        inject_b_assets: (PathBuf, PathBuf),
        inject_b_hashes: (AResult<Sha1Hash>, AResult<Sha1Hash>),
    }
    
    #[before_all]
    fn setup() -> (Arc<Context>, ()){
        let (tmp_fixtures_dir, tmp_fixtures_dir_path) = crate::tests::make_temp_fixtures(Some("bbspack"));
        let bbscript_ref = tmp_fixtures_dir_path.join("BBS_FAU.ref.bbscript");
        let inject_a_assets = (
            tmp_fixtures_dir_path.join("BBS_FAU.inject_a.uexp"),
            tmp_fixtures_dir_path.join("BBS_FAU.inject_a.uasset")
        );
        let inject_a_hashes = (sha1_hash_file(&inject_a_assets.0), sha1_hash_file(&inject_a_assets.1));
        let inject_b_assets = (
            tmp_fixtures_dir_path.join("BBS_FAU.inject_b.uexp"),
            tmp_fixtures_dir_path.join("BBS_FAU.inject_b.uasset")
        );
        let inject_b_hashes = (sha1_hash_file(&inject_b_assets.0), sha1_hash_file(&inject_b_assets.1));
        
        (Arc::new(Context { 
            _fixtures_dir: tmp_fixtures_dir,
            fixtures_dir_path: tmp_fixtures_dir_path.clone(),
            bbscript_ref,
            inject_a_assets,
            inject_a_hashes,
            inject_b_assets,
            inject_b_hashes
        }), ())
    }
    
    #[test]
    fn can_extract_uexp_bytes(ctx: Arc<Context>) {
        let uexp_path = ctx.fixtures_dir_path.join("BBS_FAU.ref.uexp");
        let expected_file_path = ctx.fixtures_dir_path.join("BBS_FAU.ref.bbscript");     
        let uexp_bytes = std::fs::read(uexp_path).unwrap();
        
        let result = super::extract_bytes(&uexp_bytes).unwrap();
        
        assert_eq!(Some(sha1_hash_bytes(&result)), sha1_hash_file(expected_file_path).ok());
    }
    
    #[test]
    fn can_extract_uexp(ctx: Arc<Context>) {
        let uexp_path = ctx.fixtures_dir_path.join("BBS_FAU.ref.uexp");
        let out_file = ctx.fixtures_dir_path.join("BBS_FAU.extracted.bbscript");
        let expected_file_path = ctx.fixtures_dir_path.join("BBS_FAU.ref.bbscript");     
        
        let result = super::extract(&uexp_path, &out_file);
        result.unwrap();
        assert!(std::fs::exists(&out_file).unwrap());
        assert_eq!(sha1_hash_file(&out_file).ok(), sha1_hash_file(expected_file_path).ok());
        
        let _ = std::fs::remove_file(out_file);
    }
    
    #[test]
    fn can_inject_bytes(ctx: Arc<Context>) {
        let bbscript_ref_bytes = std::fs::read(&ctx.bbscript_ref).unwrap();
        
        // Inject A: Should be the same hash after injection
        let mut uexp_bytes = std::fs::read(&ctx.inject_a_assets.0).unwrap();
        let mut uasset_bytes = std::fs::read(&ctx.inject_a_assets.1).unwrap();
        let result = super::inject_bytes(&bbscript_ref_bytes, &mut uexp_bytes, &mut uasset_bytes);
        result.unwrap();
        assert_eq!(ctx.inject_a_hashes.0.as_ref().ok(), Some(&sha1_hash_bytes(&uexp_bytes)));
        assert_eq!(ctx.inject_a_hashes.1.as_ref().ok(), Some(&sha1_hash_bytes(&uasset_bytes)));
        
        // Inject B: Should be a different hash after injection
        let mut uexp_bytes = std::fs::read(&ctx.inject_b_assets.0).unwrap();
        let mut uasset_bytes = std::fs::read(&ctx.inject_b_assets.1).unwrap();
        let result = super::inject_bytes(&bbscript_ref_bytes, &mut uexp_bytes, &mut uasset_bytes);
        result.unwrap();
        assert_ne!(ctx.inject_b_hashes.0.as_ref().ok(), Some(&sha1_hash_bytes(&uexp_bytes)));
        assert_ne!(ctx.inject_b_hashes.1.as_ref().ok(), Some(&sha1_hash_bytes(&uasset_bytes)));   
    }
    
    #[test]
    fn can_inject_into_uexp(ctx: Arc<Context>) {
        // Inject A: Should be the same hash after injection
        let result = super::inject(&ctx.bbscript_ref, &ctx.inject_a_assets.0, &ctx.inject_a_assets.1);
        result.unwrap();
        assert_eq!(ctx.inject_a_hashes.0.as_ref().ok(), sha1_hash_file(&ctx.inject_a_assets.0).as_ref().ok());
        assert_eq!(ctx.inject_a_hashes.1.as_ref().ok(), sha1_hash_file(&ctx.inject_a_assets.1).as_ref().ok());
        
        // Inject B: Should be a different hash after injection
        let result = super::inject(&ctx.bbscript_ref, &ctx.inject_b_assets.0, &ctx.inject_b_assets.1);
        result.unwrap();
        assert_ne!(ctx.inject_b_hashes.0.as_ref().ok(), sha1_hash_file(&ctx.inject_b_assets.0).as_ref().ok());
        assert_ne!(ctx.inject_b_hashes.1.as_ref().ok(), sha1_hash_file(&ctx.inject_b_assets.1).as_ref().ok());
    }
}

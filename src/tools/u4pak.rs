use walkdir::{WalkDir};

use crate::path::Path;

use super::*;

declare_tool!(U4PAK);


pub fn pack(dest_pak_path: impl Path, root_path: impl Path) -> ToolResult {
    let result = U4PAK!("pack", &dest_pak_path.as_path(), 
        format!(":none,rename=/RED:{}/RED", root_path.as_path().display()), 
        "--mount-point=../../..", 
        "--version=3"
    )?;
    
    if !validate(&dest_pak_path.as_path(), &root_path.as_path()) {
        return Err(anyhow::Error::msg(format!("{}'s content does not match {}'s content", 
                dest_pak_path.as_path().display(), 
                root_path.as_path().display()
            )
        ));
    }
    
    Ok(result)
}

pub fn unpack(pak_path: impl Path, out_dir: impl Path) -> ToolResult {
    let result = U4PAK!("unpack", pak_path.as_path(), "--outdir", out_dir.as_path())?;
    
    if !validate(&pak_path.as_path(), &out_dir.as_path()) {
        return Err(anyhow::Error::msg(format!("{}'s content does not match {}'s content", 
                out_dir.as_path().display(), 
                pak_path.as_path().display()
            )
        ));
    }
    
    Ok(result)
}

pub fn check(path: impl Path) -> bool {
    if let Ok(output) = U4PAK!("check", path.as_path()) {
        output == "All ok"
    } else {
        false
    }
}

pub fn list(path: impl Path) -> anyhow::Result<Vec<(String, PathBuf)>> {
    let cmd_result = U4PAK!("list", path.as_path())?;
    
    let mut list = vec![];
    let mut it = cmd_result.split_whitespace().peekable();
    while let Some(line) = it.next() {
        if let Some(path) = it.peek() && path.starts_with("RED/") {
            let hash = line.to_string();
            list.push((hash, PathBuf::from(path)));
            it.next();
        }
    }
    
    Ok(list)
}

pub fn validate(pak_path: impl Path, dir_path: impl Path) -> bool {
    if let Ok(mut pak_list) = list(&pak_path.as_path()) && check(&pak_path.as_path()) {
        let mut dir_files: Vec<(String, PathBuf)> = WalkDir::new(&dir_path.as_path()).into_iter()
            .filter_map(|e| e.ok().take_if(|e| e.file_type().is_file()))
            .map(|e| {
                let full_path = e.into_path();
                let rel_path = full_path.strip_prefix(&dir_path.as_path()).unwrap().to_path_buf();
                let hash = crate::util::sha1_hash(full_path).unwrap();
                
                (hash, rel_path)
            })
            .collect();
        pak_list.sort();
        dir_files.sort();
        
        return pak_list == dir_files;
    }
    
    false
}


#[cfg(test)]
use suitest::{suite, suite_cfg};

#[cfg(test)]
#[suite(tools_u4pak_rs)]
#[suite_cfg(sequential = true, verbose = false)]
mod tests {
    use std::{io::Write, path::PathBuf, sync::Arc};
    use suitest::before_all;
    
    
    #[derive(Debug)]
    struct Context {
        _fixtures_dir: tempfile::TempDir,
        fixtures_dir_path: PathBuf,
        pakchunk_dir_path: PathBuf,
        pakchunk_path: PathBuf
    }
    
    #[before_all]
    fn setup() -> (Arc<Context>, ()){
        let (tmp_fixtures_dir, tmp_fixtures_dir_path) = crate::tests::make_temp_fixtures(Some("u4pak"));
        
        (Arc::new(Context { 
            _fixtures_dir: tmp_fixtures_dir,
            fixtures_dir_path: tmp_fixtures_dir_path.clone(),
            pakchunk_dir_path: tmp_fixtures_dir_path.join("pakchunk"),
            pakchunk_path: tmp_fixtures_dir_path.join("pakchunk.pak")
        }), ())
    }
    
    #[test]
    fn can_list_pak_file(ctx: Arc<Context>) {
        let mut expected: Vec<(String, PathBuf)> = vec![
            (String::from("8abc079520c2e28c7385e34ac1cd1e4e8fb31056"), "RED/Content/Localization/INT/REDGame.uasset".into()),
            (String::from("c7324db8d442f9607b9118550976da84e080e204"), "RED/Content/Chara/FAU/Common/Data/BBS_FAUEF.uasset".into()),
            (String::from("af3c7f6d712c520ba490a8fe49e3c2b99b982ef7"), "RED/Content/Chara/FAU/Common/Data/BBS_FAU.uasset".into()),
            (String::from("d31b00f7283e6d10fba36d85fc6d230a84141c8b"), "RED/Content/Chara/FAU/Common/Data/BBS_FAU_BOSS.uasset".into()),
            (String::from("5baff11588777b63891d32d49ddc4c411bf107be"), "RED/Content/Chara/FAU/Common/Data/409/BBS_FAUEF.uasset".into()),
            (String::from("a95e526c1a11c697fab26e4c470e45a3f662978d"), "RED/Content/Chara/FAU/Common/Data/409/BBS_FAU.uasset".into()),
            (String::from("11d332b18aeeca51b26c964f1eea8f612404cd79"), "RED/Content/Chara/FAU/Common/Data/409/BBS_FAU_BOSS.uasset".into()),
            (String::from("64a5fff0949345c106393f40aebc58e28fd105cb"), "RED/Content/Chara/FAU/Common/Data/409/COL_FAU.uasset".into()),
            (String::from("c5f428058f13334eb380e50ed549a61b1ffb710f"), "RED/Content/Chara/FAU/Common/Data/409/BBS_FAUEF_BOSS.uasset".into()),
            (String::from("c118d88db7356cd9d1e1309814bc6c84cdd584c4"), "RED/Content/Chara/FAU/Common/Data/BBS_FAUEF_BOSS.uasset".into()),
            (String::from("4f471984360146fd4d1f4dd0be8368774ee52372"), "RED/Content/Chara/FAU/Common/Data/COL_FAU.uasset".into()),
            (String::from("130479f6d5d56fa1d32a58d80371f888541f6917"), "RED/Content/Chara/FAU/Common/Data/BBS_FAUEF.uexp".into()),
            (String::from("b637fc816934b0fbc59de9c0c06d202003a8be69"), "RED/Content/Chara/FAU/Common/Data/409/BBS_FAUEF.uexp".into()),
            (String::from("4ba11921843f01d9ddf8a076f87d50e09d6c373e"), "RED/Content/Chara/FAU/Common/Data/BBS_FAU_BOSS.uexp".into()),
            (String::from("ecec9c7adcd6f8be9d8002652763ccb7cb94e89a"), "RED/Content/Chara/FAU/Common/Data/409/BBS_FAUEF_BOSS.uexp".into()),
            (String::from("c16fec016fa054a0c3a64f95cd110af809355408"), "RED/Content/Chara/FAU/Common/Data/BBS_FAUEF_BOSS.uexp".into()),
            (String::from("e83c6faec591e041b7bd2e8a17b9d44a03a643ff"), "RED/Content/Chara/FAU/Common/Data/409/BBS_FAU.uexp".into()),
            (String::from("35ab98ed0d4b3fc90563b777e70e3ae836ed2281"), "RED/Content/Chara/FAU/Common/Data/409/BBS_FAU_BOSS.uexp".into()),
            (String::from("018b6e60e3f01bdefe2a3b512e9248dec1cd8e1c"), "RED/Content/Chara/FAU/Common/Data/BBS_FAU.uexp".into()),
            (String::from("991d9d84143d561f5c46bb035f56ced4863b6191"), "RED/Content/Chara/FAU/Common/Data/COL_FAU.uexp".into()),
            (String::from("e07fac2bce905e21fc74322cd7c6c4e7b149b601"), "RED/Content/Chara/FAU/Common/Data/409/COL_FAU.uexp".into()),
            (String::from("d988c554f720aa7bbb7e8d832db52fcbbd5a2cfb"), "RED/Content/Localization/INT/REDGame.uexp".into()),
        ];

        let mut list = super::list(&ctx.pakchunk_path).expect("Something went wrong with the list command");
        expected.sort();
        list.sort();
        
        assert_eq!(expected, list);
    }
    
    static PAK_MAGIC: u32 = 0x5A6F12E1;
    #[test]
    fn check_succeeds_on_uncorrupted_file(ctx: Arc<Context>) {
        assert!(super::check(&ctx.pakchunk_path))
    }
    #[test]
    fn check_fails_on_corrupted_file(ctx: Arc<Context>) {
        let corrupted_file_path = ctx.fixtures_dir_path.join("corrupted.pak");
        let mut file = std::fs::File::create(&corrupted_file_path).unwrap();
        file.write_all(&PAK_MAGIC.to_le_bytes()).unwrap();
        
        // Garbage
        file.write_all(&[0u8; 1024]).unwrap();

        // Pak Footer
        // [Index Offset (8b)] [Index Size (8b)] [Index Hash (20b)] [Padding (1b)] [Magic (4b)] [Version (4b)]
        file.write_all(&[0u8; 8]).unwrap();
        file.write_all(&[0u8; 8]).unwrap();
        file.write_all(&[0u8; 20]).unwrap();
        file.write_all(&[0u8; 1]).unwrap();
        file.write_all(&PAK_MAGIC.to_le_bytes()).unwrap();
        file.write_all(&8i32.to_le_bytes()).unwrap();
        
        
        assert!(!super::check(&corrupted_file_path));
        
        let _ = std::fs::remove_file(corrupted_file_path);
    }
    #[test]
    fn check_fails_on_random_file(ctx: Arc<Context>) {
        let random_file_path = ctx.fixtures_dir_path.join("not_a_pak_file.txt");
        std::fs::write(&random_file_path, b"NOT_A_PAK_FILE").unwrap();
        assert!(!super::check(&random_file_path));
        let _ = std::fs::remove_file(random_file_path);
        
        let random_file_path = ctx.fixtures_dir_path.join("not_a_pak_file.pak");
        std::fs::write(&random_file_path, b"NOT_A_PAK_FILE").unwrap();
        assert!(!super::check(&random_file_path));
        let _ = std::fs::remove_file(random_file_path);
        
    }
    
    
    #[test]
    fn validate_succeeds_when_pak_content_matches_dir_content(ctx: Arc<Context>) {
        let result = super::validate(&ctx.pakchunk_path, &ctx.pakchunk_dir_path);
        assert!(result);
    }
    #[test]
    fn validate_fails_when_pak_content_doesnt_match_dir_content(ctx: Arc<Context>) {
        let foo_file = ctx.pakchunk_dir_path.join("foo.bar");
        std::fs::write(&foo_file, b"foo_bar").unwrap();
        
        let result = super::validate(&ctx.pakchunk_path, &ctx.pakchunk_dir_path);
        assert!(!result);
        
        let _ = std::fs::remove_file(&foo_file);
    }
    
    #[test]
    fn pack_works(ctx: Arc<Context>) {
        let packed_path = ctx.fixtures_dir_path.join("pack_test.pak");
        let result = super::pack(&packed_path, &ctx.pakchunk_dir_path);
        result.unwrap();
        
        let _ = std::fs::remove_file(packed_path);
    }
    
    #[test]
    fn unpack_works(ctx: Arc<Context>) {
        let unpacked_path = ctx.fixtures_dir_path.join("unpacked");
        let result = super::unpack(&ctx.pakchunk_path, &unpacked_path);
        result.unwrap();
        
        let _ = std::fs::remove_file(unpacked_path);
    }
}

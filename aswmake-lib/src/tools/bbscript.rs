use crate::path::Path;

use super::*;

pub fn parse(bbscript_bin_path: impl Path, out_file: impl Path, game: crate::TargetGame) -> AResult<()> {
    let bin_path = bbscript_bin_path.absolute_file().or(Err(crate::error::InvalidFilePath(bbscript_bin_path)))?;
    let out_file = out_file.absolute().or(Err(crate::error::InvalidFilePath(out_file)))?;
    crate::bbscript::parse(
        game.to_supported_game(),
        &mut std::fs::File::open(bin_path)?,
        &mut std::fs::File::create(out_file)?,
        None, None,
        false,
        12
    )?;
    Ok(())
}

pub fn parse_bytes<R>(mut uexp_bytes: R, game: crate::TargetGame) -> AResult<Vec<u8>>
where R: std::io::Read {
    crate::bbscript::parse_bytes(
        game.to_supported_game(),
        &mut uexp_bytes,
        None, None,
        false,
        12
    )
}

pub fn rebuild(input_file: impl Path, out_bbscript_bin_path: impl Path, game: crate::TargetGame) -> AResult<()> {
    let input_file = input_file.absolute_file().or(Err(crate::error::InvalidFilePath(input_file)))?;
    let out_bin = out_bbscript_bin_path.absolute().or(Err(crate::error::InvalidFilePath(out_bbscript_bin_path)))?;
    
    crate::bbscript::rebuild(
        game.to_supported_game(), 
        std::fs::File::open(input_file)?, 
        &mut std::fs::File::create(out_bin)?, 
        false
    )?;
    Ok(())
}

pub fn rebuild_bytes<R: std::io::Read>(input_bytes: R, game: crate::TargetGame) -> AResult<Vec<u8>> {
    crate::bbscript::rebuild_bytes(
        game.to_supported_game(), 
        input_bytes, 
        false
    )
}

#[cfg(test)]
use suitest::{suite, suite_cfg};

#[cfg(test)]
#[suite(tools_bbscript_rs)]
#[suite_cfg(sequential = true, verbose = false)]
mod tests {
    use std::{path::PathBuf, sync::Arc};
    use suitest::before_all;
    use crate::util::hashid_from_file;
    
    #[derive(Debug)]
    struct Context {
        _fixtures_dir: tempfile::TempDir,
        fixtures_dir_path: PathBuf
    }
    
    #[before_all]
    fn setup() -> (Arc<Context>, ()){
        let (tmp_fixtures_dir, tmp_fixtures_dir_path) = crate::tests::make_temp_fixtures(Some("bbscript"));
        
        (Arc::new(Context { 
            _fixtures_dir: tmp_fixtures_dir,
            fixtures_dir_path: tmp_fixtures_dir_path.clone()
        }), ())
    }
    
    #[test]
    fn can_parse_bbscript(ctx: Arc<Context>) {
        let bbscript_path = ctx.fixtures_dir_path.join("BBS_FAU.ref.bbscript");
        let out_file = ctx.fixtures_dir_path.join("BBS_FAU.parsed.bbs");
        let expected_file_path = ctx.fixtures_dir_path.join("BBS_FAU.ref.bbs");
        
        let result = super::parse(&bbscript_path, &out_file, crate::TargetGame::GGST);
        result.unwrap();
        assert!(std::fs::exists(&out_file).unwrap());
        assert_eq!(hashid_from_file(&out_file).ok(), hashid_from_file(expected_file_path).ok());
        
        let _ = std::fs::remove_file(out_file);
    }
    
    #[test]
    fn can_rebuild_human_readable_into_bbscript(ctx: Arc<Context>) {
        let bbscript_hr_ref = ctx.fixtures_dir_path.join("BBS_FAU.ref.bbs");
        let out_file = ctx.fixtures_dir_path.join("BBS_FAU.rebuilt.bbscript");
        let expected_file_path = ctx.fixtures_dir_path.join("BBS_FAU.ref.bbscript");
        
        let result = super::rebuild(&bbscript_hr_ref, &out_file, crate::TargetGame::GGST);
        result.unwrap();
        assert!(std::fs::exists(&out_file).unwrap());
        assert_eq!(hashid_from_file(&out_file).ok(), hashid_from_file(expected_file_path).ok());
    }
}

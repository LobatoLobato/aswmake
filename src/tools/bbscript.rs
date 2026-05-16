use crate::path::Path;

use super::*;

declare_tool!(BBSCRIPT);

pub fn parse(bbscript_bin_path: impl Path, out_file: impl Path, game: crate::TargetGame) -> ToolResult {
    BBSCRIPT!("parse", "--overwrite", "--game", game.lcname(), bbscript_bin_path.as_path(), out_file.as_path())
}

pub fn rebuild(input_file: impl Path, out_bbscript_bin_path: impl Path, game: crate::TargetGame) -> ToolResult {
    BBSCRIPT!("rebuild", "-o", "--game", game.lcname(), input_file.as_path(), out_bbscript_bin_path.as_path())
}

#[cfg(test)]
use suitest::{suite, suite_cfg};

#[cfg(test)]
#[suite(tools_bbscript_rs)]
#[suite_cfg(sequential = true, verbose = false)]
mod tests {
    use std::{path::PathBuf, sync::Arc};
    use suitest::before_all;
    use crate::util::sha1_hash;
    
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
        assert_eq!(sha1_hash(&out_file).ok(), sha1_hash(expected_file_path).ok());
        
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
        assert_eq!(sha1_hash(&out_file).ok(), sha1_hash(expected_file_path).ok());
    }
}

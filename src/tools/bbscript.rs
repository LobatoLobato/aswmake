use strum::{EnumString, Display, IntoStaticStr, VariantNames, EnumProperty};

use crate::path::Path;

use super::*;

declare_tool!(BBSCRIPT);

#[derive(Debug, Clone, EnumString, Display, IntoStaticStr, VariantNames, EnumProperty)]
#[strum(serialize_all = "lowercase")]
pub enum TargetGame {
    #[strum(props(aes_key = ""))]
    BBCF,
    #[strum(props(aes_key = ""))]
    DBFZ,
    #[strum(props(aes_key = ""))]
    DNF,
    #[strum(props(aes_key = ""))]
    GBVS,
    #[strum(props(aes_key = ""))]
    GBVSR,
    #[strum(props(aes_key = ""))]
    GGREV2,
    #[strum(props(aes_key = "0x3D96F3E41ED4B90B6C96CA3B2393F8911A5F6A48FE71F54B495E8F1AFD94CD73"))]
    GGST,
    #[strum(props(aes_key = ""))]
    P4U2
}

impl TargetGame {
    fn lcname(&self) -> String {
        return format!("{self:?}").to_lowercase();
    }
    pub fn aes_key(&self) -> &str {
        self.get_str("aes_key").unwrap_or_default()
    }
}

pub fn parse(bbscript_bin_path: impl Path, out_file: impl Path, game: TargetGame) -> ToolResult {
    BBSCRIPT!("parse", "--overwrite", "--game", game.lcname(), bbscript_bin_path.as_path(), out_file.as_path())
}

pub fn rebuild(input_file: impl Path, out_bbscript_bin_path: impl Path, game: TargetGame) -> ToolResult {
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
        
        let result = super::parse(&bbscript_path, &out_file, super::TargetGame::GGST);
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
        
        let result = super::rebuild(&bbscript_hr_ref, &out_file, super::TargetGame::GGST);
        result.unwrap();
        assert!(std::fs::exists(&out_file).unwrap());
        assert_eq!(sha1_hash(&out_file).ok(), sha1_hash(expected_file_path).ok());
    }
}

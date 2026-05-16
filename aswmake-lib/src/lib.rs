pub mod tools;
pub mod parsers;
pub mod util;
pub mod path;
pub mod build;
pub mod error;

use strum::{EnumString, Display, IntoStaticStr, VariantNames, EnumProperty};
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

#[cfg(test)]
pub mod tests {
    pub fn make_temp_fixtures(unit: Option<&str>) -> (tempfile::TempDir, std::path::PathBuf) {
        use fs_extra::dir::{copy, CopyOptions};
        use std::path::PathBuf;
        let tmp_fixtures_dir = tempfile::tempdir().expect("Could not create temp dir for loc.rs tests");
        let tmp_fixtures_dir_path = tmp_fixtures_dir.path().to_path_buf();
        
        let mut fixtures_path = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures"));
        if let Some(unit) = unit {
            fixtures_path = fixtures_path.join(unit);
        }
        
        copy(fixtures_path, &tmp_fixtures_dir, &CopyOptions::new().content_only(true))
            .expect("Could not copy fixtures into temp dir");
        
        (tmp_fixtures_dir, tmp_fixtures_dir_path)
    }
}

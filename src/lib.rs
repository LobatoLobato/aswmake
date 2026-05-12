pub mod tools;
pub mod loc;
pub mod util;

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

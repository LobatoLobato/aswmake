pub mod tools;
pub mod loc;

#[cfg(test)]
pub mod tests {
    pub fn make_temp_fixtures() -> (tempfile::TempDir, std::path::PathBuf) {
        use fs_extra::dir::{copy, CopyOptions};
        let fixtures_path = std::env::var("FIXTURES_DIR").expect("Missing FIXTURES_DIR env var");
        let tmp_fixtures_dir = tempfile::tempdir().expect("Could not create temp dir for loc.rs tests");
        let tmp_fixtures_dir_path = tmp_fixtures_dir.path().to_path_buf();
        
        copy(fixtures_path, &tmp_fixtures_dir, &CopyOptions::new().content_only(true)).expect("Could not copy fixtures into temp dir");
        (tmp_fixtures_dir, tmp_fixtures_dir_path)
    }
}

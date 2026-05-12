use std::sync::LazyLock;

use super::*;

declare_tool!(QUICKBMS);

static BMS_SCRIPT: LazyLock<tempfile::NamedTempFile> = LazyLock::new(|| {
    let bms_script = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/external/quickbms/ggst.bms"));
    let bms_script_file = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(bms_script_file.path(), bms_script).unwrap(); 
    
    bms_script_file
});


pub fn extract(pak_path: impl AsRef<Path>, out_dir: impl AsRef<Path>, filters: Option<&[&str]>) -> ToolResult {
    let filters = filters.map(|f| format!("-f \"{}\"", f.join(";"))).unwrap_or(String::new());
    QUICKBMS!("-o", "-Y", filters, BMS_SCRIPT.path(), pak_path, out_dir)
}


#[cfg(test)]
use suitest::{suite, suite_cfg};

#[cfg(test)]
#[suite(tools_quickbms_rs)]
#[suite_cfg(sequential = true, verbose = false)]
mod tests {
    use std::{path::PathBuf, sync::Arc};
    use suitest::before_all;
    
    #[derive(Debug)]
    struct Context {
        _fixtures_dir: tempfile::TempDir,
        fixtures_dir_path: PathBuf,
        pakchunk_path: PathBuf
    }
    
    #[before_all]
    fn setup() -> (Arc<Context>, ()){
        let (tmp_fixtures_dir, tmp_fixtures_dir_path) = crate::tests::make_temp_fixtures(Some("quickbms"));
        
        (Arc::new(Context { 
            _fixtures_dir: tmp_fixtures_dir,
            fixtures_dir_path: tmp_fixtures_dir_path.clone(),
            pakchunk_path: tmp_fixtures_dir_path.join("pakchunk.pak")
        }), ())
    }
    
    #[test]
    fn can_extract_pak(ctx: Arc<Context>) {
        let pakchunk_dir_path = ctx.fixtures_dir_path.join("pakchunk");
        let out_dir = ctx.fixtures_dir_path.join("extracted");
        let result = super::quickbms::extract(&ctx.pakchunk_path, &out_dir, None);
        result.unwrap();
        
        assert!(!dir_diff::is_different(&pakchunk_dir_path, &out_dir).unwrap());   
        
        let _ = std::fs::remove_dir_all(out_dir);
    }
    
    #[test]
    fn can_extract_specific_paths_inside_pak(ctx: Arc<Context>) {
        let filtered_pakchunk_dir_path = ctx.fixtures_dir_path.join("pakchunk-filtered");
        let out_dir = ctx.fixtures_dir_path.join("extracted");
        let result = super::quickbms::extract(&ctx.pakchunk_path, &out_dir, Some(&[
            "{}/Localization/{}.uasset", 
            "{}/COL{}.uexp"
        ]));
        result.unwrap();
        
        assert!(!dir_diff::is_different(&filtered_pakchunk_dir_path, &out_dir).unwrap());   
        
        let _ = std::fs::remove_dir_all(out_dir);
    }
}

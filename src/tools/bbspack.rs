use super::*;

declare_tool!(BBSPACK);

pub fn extract(uexp: impl AsRef<Path>, out_file: impl AsRef<Path>) -> ToolResult {
    BBSPACK!("extract", uexp, out_file)
}

pub fn inject(input_file: impl AsRef<Path>, uexp: impl AsRef<Path>, uasset: impl AsRef<Path>) -> ToolResult {
    BBSPACK!("inject", input_file, uexp, uasset)
}


#[cfg(test)]
use suitest::{suite, suite_cfg};

#[cfg(test)]
#[suite(tools_bbspack_rs)]
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
        let (tmp_fixtures_dir, tmp_fixtures_dir_path) = crate::tests::make_temp_fixtures(Some("bbspack"));
        
        (Arc::new(Context { 
            _fixtures_dir: tmp_fixtures_dir,
            fixtures_dir_path: tmp_fixtures_dir_path.clone()
        }), ())
    }
    
    #[test]
    fn can_extract_uexp(ctx: Arc<Context>) {
        let uexp_path = ctx.fixtures_dir_path.join("BBS_FAU.ref.uexp");
        let out_file = ctx.fixtures_dir_path.join("BBS_FAU.extracted.bbscript");
        let expected_file_path = ctx.fixtures_dir_path.join("BBS_FAU.ref.bbscript");     
        
        let result = super::extract(&uexp_path, &out_file);
        result.unwrap();
        assert!(std::fs::exists(&out_file).unwrap());
        assert_eq!(sha1_hash(&out_file).ok(), sha1_hash(expected_file_path).ok());
        
        let _ = std::fs::remove_file(out_file);
    }
    
    #[test]
    fn can_inject_into_uexp(ctx: Arc<Context>) {
        let bbscript_ref = ctx.fixtures_dir_path.join("BBS_FAU.ref.bbscript");
        let (inject_a_uexp, inject_a_uasset) = (
            ctx.fixtures_dir_path.join("BBS_FAU.inject_a.uexp"),
            ctx.fixtures_dir_path.join("BBS_FAU.inject_a.uasset")
        );
        let (inject_a_uexp_hash, inject_a_uasset_hash) = (sha1_hash(&inject_a_uexp), sha1_hash(&inject_a_uasset));
        let (inject_b_uexp, inject_b_uasset) = (
            ctx.fixtures_dir_path.join("BBS_FAU.inject_b.uexp"),
            ctx.fixtures_dir_path.join("BBS_FAU.inject_b.uasset")
        );
        let (inject_b_uexp_hash, inject_b_uasset_hash) = (sha1_hash(&inject_b_uexp), sha1_hash(&inject_b_uasset));
        
        // Inject A: Should be the same hash after injection
        let result = super::inject(&bbscript_ref, &inject_a_uexp, &inject_a_uasset);
        result.unwrap();
        assert_eq!(inject_a_uexp_hash.ok(), sha1_hash(inject_a_uexp).ok());
        assert_eq!(inject_a_uasset_hash.ok(), sha1_hash(inject_a_uasset).ok());
        
        // Inject B: Should be a different hash after injection
        let result = super::inject(&bbscript_ref, &inject_b_uexp, &inject_b_uasset);
        result.unwrap();
        assert_ne!(inject_b_uexp_hash.ok(), sha1_hash(inject_b_uexp).ok());
        assert_ne!(inject_b_uasset_hash.ok(), sha1_hash(inject_b_uasset).ok());   
    }
}

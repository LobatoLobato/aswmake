use crate::path::Path;

pub fn sha1_hash_bytes(buffer: &Vec<u8>) -> anyhow::Result<String> {
    use sha1::{Sha1, Digest};
    let mut hasher = Sha1::new();
    hasher.update(&buffer);
    
    Ok(hex::encode(hasher.finalize()))
}

pub fn sha1_hash(path: impl Path) -> anyhow::Result<String> {
    use sha1::{Sha1, Digest};
    use std::io::Read;
    let mut file = std::fs::File::open(path.as_path())?;
    let mut hasher = Sha1::new();
    let mut buffer = [0u8; 8192];
    
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 { break; }
        hasher.update(&buffer[..count]);
    }
    
    let hash = hasher.finalize();
    
    Ok(hash.iter().map(|b| format!("{:02x}", b)).collect())
}

macro_rules! make {
    ($type:ident { $($field:ident : $val:expr),* $(,)? }) => {
            $type {
                $($field: $val,)*
                ..Default::default()
            }
        };
}
pub(crate) use make;

#[cfg(test)]
use suitest::{suite, suite_cfg};
#[cfg(test)]
#[suite(util_rs)]
#[suite_cfg(sequential = true, verbose = false)]
mod tests {
    use std::{path::PathBuf, sync::Arc};
    use suitest::before_all;
    
    
    #[derive(Debug)]
    struct Context {
        _fixtures_dir: tempfile::TempDir,
        fixtures_dir_path: PathBuf
    }
    
    #[before_all]
    fn setup() -> (Arc<Context>, ()){
        let (tmp_fixtures_dir, tmp_fixtures_dir_path) = crate::tests::make_temp_fixtures(None);
        
        (Arc::new(Context { 
            _fixtures_dir: tmp_fixtures_dir,
            fixtures_dir_path: tmp_fixtures_dir_path,
        }), ())
    }
    
    #[test]
    fn can_generate_sha1_hash_of_file(ctx: Arc<Context>) {
        let expected_hash = String::from("94aed695c46cf1606cfbc9c2b66c7205ed55b9ad");
        let test_file_path = ctx.fixtures_dir_path.join("sha1hash.test");
        std::fs::write(&test_file_path, "sha1 hash test").unwrap();
        
        assert_eq!(super::sha1_hash(&test_file_path).ok(), Some(expected_hash));
        
        let _ = std::fs::remove_file(test_file_path);
    }
}
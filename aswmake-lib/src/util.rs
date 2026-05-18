use crate::{AResult, path::Path};

pub type Sha1Hash = [u8; 20];

pub fn sha1_hash_bytes(buffer: &[u8]) -> Sha1Hash {
    use sha1::{Sha1, Digest};
    let mut hasher = Sha1::new();
    hasher.update(buffer);
    
    hasher.finalize().into()
}

pub fn sha1_hash_reader<R: std::io::Read>(mut reader: R) -> AResult<Sha1Hash> {
    use sha1::{Sha1, Digest};
    let mut hasher = Sha1::new();
    let mut buffer = [0; 8192];

    loop {
        let read_bytes = reader.read(&mut buffer)?;
        if read_bytes == 0 { break; }
        hasher.update(&buffer[..read_bytes]);
    }

    Ok(hasher.finalize().into())
}

pub fn sha1_hash_file(path: impl Path) -> AResult<Sha1Hash> {
    use std::io::BufReader;
    let file = std::fs::File::open(path.as_path())?;
    let buf_reader = BufReader::new(file);
    sha1_hash_reader(buf_reader)
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
        let expected_hash = hex::decode("94aed695c46cf1606cfbc9c2b66c7205ed55b9ad").unwrap();
        let test_file_path = ctx.fixtures_dir_path.join("sha1hash.test");
        std::fs::write(&test_file_path, "sha1 hash test").unwrap();
        
        assert_eq!(super::sha1_hash_file(&test_file_path).as_ref().ok(), expected_hash.as_array());
        
        let _ = std::fs::remove_file(test_file_path);
    }
}
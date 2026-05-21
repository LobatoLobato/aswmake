use crate::{AResult, path::Path};

pub type HashId = u64;

pub fn hashid_from_bytes(buffer: &[u8]) -> HashId {
    let mut hasher = xxhash_rust::xxh3::Xxh3::new();
    hasher.update(buffer);
    
    hasher.digest()
}

pub fn hashid_from_reader<R: std::io::Read>(mut reader: R) -> AResult<HashId> {
    let mut hasher = xxhash_rust::xxh3::Xxh3::new();
    let mut buffer = [0; 8192];

    loop {
        let read_bytes = reader.read(&mut buffer)?;
        if read_bytes == 0 { break; }
        hasher.update(&buffer[..read_bytes]);
    }

    Ok(hasher.digest())
}

pub fn hashid_from_file(path: impl Path) -> AResult<HashId> {
    use std::io::BufReader;
    let file = std::fs::File::open(path.as_path())?;
    let buf_reader = BufReader::new(file);
    hashid_from_reader(buf_reader)
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
    fn can_generate_hashid_from_file(ctx: Arc<Context>) {
        let expected_hash = 0x064e403a22e946bf; // xxh3 hash of "hash test"
        let test_file_path = ctx.fixtures_dir_path.join("hash.test");
            std::fs::write(&test_file_path, "hash test").unwrap();
        
        assert_eq!(super::hashid_from_file(&test_file_path).as_ref().ok(), Some(&expected_hash));
        
        let _ = std::fs::remove_file(test_file_path);
    }
}
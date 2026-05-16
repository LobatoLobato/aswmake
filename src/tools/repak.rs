use std::{fs::File, io::BufReader, path::PathBuf};

use aes::cipher::KeyInit as _;
use rayon::iter::{IntoParallelRefIterator as _, ParallelIterator as _};
use glob_match::glob_match;

use crate::{error, path::Path};

pub fn extract(
    game_pak_path: impl Path, 
    bms_dir: impl Path, 
    aes_key: &str,
    filters: Option<&[&str]>,
    before_hook: impl Fn(&String, &PathBuf) + Send + Sync,
    after_hook: impl Fn(&PathBuf) + Send + Sync
) -> anyhow::Result<Vec<PathBuf>> {
    let bms_dir = bms_dir.absolute().or(Err(error::InvalidFilePath(bms_dir)))?;
    let game_pak_path = game_pak_path.absolute_file().or(Err(error::InvalidFilePath(game_pak_path)))?;
    let aes_key = aes::Aes256::new_from_slice(&hex::decode(aes_key.trim_start_matches("0x"))?)?;
    let filter = |file_path: &&String| if let Some(filters) = filters {
        filters.iter().any(|filter| glob_match(filter, file_path))
    } else {
        true
    };
    
    let pak_builder = repak::PakBuilder::new().key(aes_key);
    let mut pak_bufreader = BufReader::new(File::open(game_pak_path.as_path())?);
    let pak = pak_builder.reader(&mut pak_bufreader)?;
    
    std::fs::create_dir_all(&bms_dir)?;
    
    println!("Filtering files...");
    let generated_files = pak.files().par_iter().filter(filter).map(|file_path| {
        let out_path = bms_dir.as_path().join(&file_path);
        
        before_hook(&file_path, &out_path);
        
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        let mut thread_file = std::io::BufReader::new(std::fs::File::open(&game_pak_path)?);
        let mut out_file = std::fs::File::create(&out_path)?;
        pak.read_file(&file_path, &mut thread_file, &mut out_file)?;
        
        drop(out_file);
        
        after_hook(&out_path);
        
        Ok(out_path)
    }).collect::<anyhow::Result<Vec<PathBuf>>>();
    
    generated_files
}

#[cfg(test)]
use suitest::{suite, suite_cfg};

#[cfg(test)]
#[suite(tools_repak_rs)]
#[suite_cfg(sequential = true, verbose = false)]
mod tests {
    use std::{path::PathBuf, sync::Arc};
    use suitest::before_all;

use crate::tools::bbscript::TargetGame;
    
    #[derive(Debug)]
    struct Context {
        _fixtures_dir: tempfile::TempDir,
        fixtures_dir_path: PathBuf,
        pakchunk_path: PathBuf
    }
    
    #[before_all]
    fn setup() -> (Arc<Context>, ()){
        let (tmp_fixtures_dir, tmp_fixtures_dir_path) = crate::tests::make_temp_fixtures(Some("repak"));
        
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
        let result = super::extract(&ctx.pakchunk_path, &out_dir, 
            TargetGame::GGST.aes_key(),
            None,
            |_, _| {},
            |_| {}
        );
        result.unwrap();
        
        assert!(!dir_diff::is_different(&pakchunk_dir_path, &out_dir).unwrap());   
        
        let _ = std::fs::remove_dir_all(out_dir);
    }
    
    #[test]
    fn can_extract_specific_paths_inside_pak(ctx: Arc<Context>) {
        let filtered_pakchunk_dir_path = ctx.fixtures_dir_path.join("pakchunk-filtered");
        let out_dir = ctx.fixtures_dir_path.join("extracted");
        let result = super::extract(&ctx.pakchunk_path, &out_dir, 
            TargetGame::GGST.aes_key(),
            Some(&[
                "**/Localization/**/*.uasset", 
                "**/COL*.uexp"
            ]),
            |_, _| {},
            |_| {}
        );
        result.unwrap();
        
        assert!(!dir_diff::is_different(&filtered_pakchunk_dir_path, &out_dir).unwrap());   
        
        let _ = std::fs::remove_dir_all(out_dir);
    }
}

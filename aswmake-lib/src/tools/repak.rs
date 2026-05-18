use std::{fs::File, io::BufReader, path::PathBuf, str::FromStr};

use aes::cipher::{KeyInit as _};
use itertools::Itertools;
use rayon::iter::{IntoParallelRefIterator as _, ParallelBridge as _, ParallelIterator};
use glob_match::glob_match;

use crate::{AResult, error, path::Path, util::Sha1Hash};

use strum::{EnumString};

pub use repak::Version;

#[derive(Debug, Clone, EnumString, PartialEq)]
#[strum(serialize_all = "lowercase")]
pub enum PakFileKind {
    Uexp,
    Uasset,
    
    #[strum(default)]
    Other(String)
}

pub struct PakFile<'a> {
    pub hash: Sha1Hash,
    pub path: &'a std::path::Path,
    pub size: usize,
    pub kind: PakFileKind
}


pub struct PakReader {
    pak: repak::PakReader,
    pak_path: PathBuf,
    aes_key: aes::Aes256
}

pub type SimpleEntry<'a> = (Sha1Hash, &'a std::path::Path);

impl PakReader {
    pub fn new(pak_path: impl Path, aes_key: &str) -> AResult<Self> {
        let aes_key = aes::Aes256::new_from_slice(&hex::decode(aes_key.trim_start_matches("0x"))?)?;
        Self::new_aes(pak_path, aes_key)
    }
    
    pub fn new_aes(pak_path: impl Path, aes_key: aes::Aes256) -> AResult<Self> {
        let pak_path = pak_path.absolute_file().or(Err(error::InvalidFilePath(pak_path)))?;
        
        let pak_builder = repak::PakBuilder::new().key(aes_key.clone());
        let mut pak_bufreader = Self::create_buf_reader(&pak_path)?;
        let pak = pak_builder.reader(&mut pak_bufreader)?;
        
        Ok(Self {pak, pak_path, aes_key})
    }
    
    pub fn unpack(
        &self, 
        ms_dir: impl Path, 
        filters: Option<&[&str]>,
        before_hook: impl Fn(&String, &PathBuf) + Send + Sync,
        after_hook: impl Fn(&PathBuf) + Send + Sync
    ) -> AResult<Vec<PathBuf>> {
        let ms_dir = ms_dir.absolute().or(Err(error::InvalidFilePath(ms_dir)))?;
        let filter = |file_path: &&String| if let Some(filters) = filters {
            filters.iter().any(|filter| glob_match(filter, file_path))
        } else {
            true
        };
        
        std::fs::create_dir_all(&ms_dir)?;
        
        let generated_files = self.pak.files().par_iter().filter(filter).map(|file_path| {
            let out_path = ms_dir.as_path().join(&file_path);
            
            before_hook(&file_path, &out_path);
            
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            
            let mut thread_file = std::io::BufReader::new(std::fs::File::open(&self.pak_path)?);
            let mut out_file = std::fs::File::create(&out_path)?;
            self.pak.read_file(&file_path, &mut thread_file, &mut out_file)?;
            
            drop(out_file);
            
            after_hook(&out_path);
            
            Ok(out_path)
        }).collect::<AResult<Vec<PathBuf>>>();
        
        if !self.validate(&ms_dir, filters) {
            return Err(anyhow::Error::msg(format!("{}'s content does not match {}'s content", 
                    self.pak_path.display(), 
                    ms_dir.display()
                )
            ));
        }
        
        generated_files
    }
    
    fn pack_static(
        aes_key: &str,
        version: Version,
        mount_point: &str,
        input_dir: impl Path, 
        out_pak: impl Path
    ) -> AResult<()> {
        let input_dir = input_dir.absolute_dir().or(Err(error::InvalidFilePath(input_dir)))?;
        let out_pak = out_pak.absolute().or(Err(error::InvalidFilePath(out_pak)))?;
        
        let aes_key = aes::Aes256::new_from_slice(&hex::decode(aes_key.trim_start_matches("0x"))?)?;
        
        let writer = std::io::Cursor::new(vec![]);
        let mut pak_writer = repak::PakBuilder::new().key(aes_key.clone()).writer(
            writer,
            version,
            mount_point.to_owned(),
            None,
        );
        
        for entry in walkdir::WalkDir::new(&input_dir).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                let pak_path = entry.path().strip_prefix(&input_dir)?;
                pak_writer.write_file(&pak_path.to_string_lossy(), false, std::fs::read(entry.path())?)?;
            }
        }
        
        std::fs::write(&out_pak, pak_writer.write_index().unwrap().into_inner())?;
        
        if !Self::validate(&PakReader::new_aes(&out_pak, aes_key)?, &input_dir, None) {
            return Err(anyhow::Error::msg(format!("{}'s content does not match {}'s content", 
                    out_pak.display(), 
                    input_dir.display()
                )
            ));
        }
        
        Ok(())
    } 
    pub fn pack(&self, input_dir: impl Path, out_pak: impl Path) -> AResult<()> {
        let input_dir = input_dir.absolute_dir().or(Err(error::InvalidFilePath(input_dir)))?;
        let out_pak = out_pak.absolute().or(Err(error::InvalidFilePath(out_pak)))?;
        
        let writer = std::io::Cursor::new(vec![]);
        let mut pak_writer = repak::PakBuilder::new().key(self.aes_key.clone()).writer(
            writer,
            self.pak.version(),
            self.pak.mount_point().to_owned(),
            self.pak.path_hash_seed(),
        );
        
        for entry in walkdir::WalkDir::new(&input_dir).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                let pak_path = entry.path().strip_prefix(&input_dir)?;
                pak_writer.write_file(&pak_path.to_string_lossy(), false, std::fs::read(entry.path())?)?;
            }
        }
        
        std::fs::write(&out_pak, pak_writer.write_index().unwrap().into_inner())?;
        
        if !Self::validate(&PakReader::new_aes(&out_pak, self.aes_key.clone())?, &input_dir, None) {
            return Err(anyhow::Error::msg(format!("{}'s content does not match {}'s content", 
                    out_pak.display(), 
                    input_dir.display()
                )
            ));
        }
        
        Ok(())
    }
    
    
    pub fn list<'a>(&'a self, filters: Option<&[&str]>) -> AResult<Vec<PakFile<'a>>> {
        self.list_iter(filters).par_bridge().collect()
    }
    pub fn list_simple<'a>(&'a self, filters: Option<&[&str]>) -> AResult<Vec<SimpleEntry<'a>>> {
        self.list_simple_iter(filters).par_bridge().collect()
    }
    pub fn list_simple_iter<'a>(&'a self, filters: Option<&[&str]>) -> impl Iterator<Item = AResult<SimpleEntry<'a>>> {
        self.list_iter(filters).map(|pf| pf.map(|pf| (pf.hash, pf.path)))
    }
    pub fn list_iter<'a>(&'a self, filters: Option<&[&str]>) -> impl Iterator<Item = AResult<PakFile<'a>>> {
        let filter = move |file_path: &'a String| match filters {
            Some(filters) => filters.iter().any(|filter| glob_match(filter, file_path)).then_some(file_path),
            None => Some(file_path)
        };
           
        self.pak.files_ref().into_iter().filter_map(move |file_path| filter(file_path))
            .map(move |path| -> AResult<PakFile<'_>> {
                let mut thread_file = Self::create_buf_reader(&self.pak_path)?;
                let mut buffer = vec![];
                self.pak.read_file(&path, &mut thread_file, &mut buffer)?;
                
                let hash = crate::util::sha1_hash_reader(std::io::Cursor::new(&buffer))?;
                let size = buffer.len();
                let kind = if let Some(ext) = path.as_path().extension() {
                    PakFileKind::from_str(&ext.to_string_lossy())?
                } else { 
                    PakFileKind::Other(String::new())
                };
                
                Ok(PakFile {path: path.as_path(), hash, size, kind })
            })
    }
    
    pub fn read_file(&self, file_path: impl Path) -> AResult<Vec<u8>> {
        let mut buffer = vec![];
        let mut pak_bufreader = Self::create_buf_reader(&self.pak_path)?;
        self.pak.read_file(&file_path.as_path().to_string_lossy(), &mut pak_bufreader, &mut buffer)?;
        Ok(buffer)
    }
    
    pub fn validate(&self, dir_path: impl Path, filters: Option<&[&str]>) -> bool {
        let Ok(dir_path) = dir_path.absolute_dir() else {return false};
        let filter = |file_path: &std::path::Path| filters.map(|filters| {
            filters.iter().any(|filter| glob_match(filter, &file_path.to_string_lossy()))
        }).unwrap_or(true);
        
        let pak_list = self.list_simple_iter(filters)
            .filter_map(|pf| pf.ok())
            .sorted_by(|a, b| Ord::cmp(a.1, b.1));
        
        walkdir::WalkDir::new(&dir_path.as_path()).into_iter()
            .filter_map(|e| e.ok().take_if(|e| e.file_type().is_file()))
            .filter(|e| filter(e.path())).sorted_by(|a, b| Ord::cmp(a.path(), b.path()))
            .zip_longest(pak_list).all(|z| {
                let itertools::EitherOrBoth::Both(df, pf) = z else { return false; };
                let rel_path = df.path().strip_prefix(&dir_path.as_path()).unwrap();
                let hash = crate::util::sha1_hash_file(df.path()).unwrap();
                
                pf.0 == hash && pf.1 == rel_path       
            })
    }

    fn create_buf_reader(pak_path: impl Path) -> AResult<BufReader<File>> {
        Ok(BufReader::new(File::open(pak_path.as_path())?))
    }
}

pub fn unpack(
    pak_path: impl Path,
    aes_key: &str, 
    ms_dir: impl Path, 
    filters: Option<&[&str]>,
    before_hook: impl Fn(&String, &PathBuf) + Send + Sync,
    after_hook: impl Fn(&PathBuf) + Send + Sync
) -> AResult<Vec<PathBuf>> {
    PakReader::new(pak_path, aes_key)?.unpack(ms_dir, filters, before_hook, after_hook)
}

pub fn pack(
    aes_key: &str, 
    version: Version, 
    mount_point: &str, 
    input_dir: impl Path, 
    out_pak: impl Path
) -> AResult<()> {
    PakReader::pack_static(aes_key, version, mount_point, input_dir, out_pak)
}

// pub fn list<'a>(pak_path: impl Path, aes_key: &str, filters: Option<&[&str]>) -> AResult<Vec<PakFile2<'a>>> {
//     PakReader::new(pak_path, aes_key)?.list(filters)
// }

pub fn read_file(pak_path: impl Path, aes_key: &str, file_path: impl Path) -> AResult<Vec<u8>> {
    PakReader::new(pak_path, aes_key)?.read_file(file_path)
}

pub fn validate(pak_path: impl Path, aes_key: &str, dir_path: impl Path, filters: Option<&[&str]>) -> bool {
    PakReader::new(pak_path, aes_key).is_ok_and(|r| r.validate(dir_path, filters))
}

#[cfg(test)]
use suitest::{suite, suite_cfg};

#[cfg(test)]
#[suite(tools_repak_rs)]
#[suite_cfg(sequential = true, verbose = false)]
mod tests {
    use std::{path::PathBuf, sync::Arc};
    use suitest::before_all;

    use crate::util::sha1_hash_file;
    
    #[derive(Debug)]
    struct Context {
        _fixtures_dir: tempfile::TempDir,
        fixtures_dir_path: PathBuf,
        pakchunk_path: PathBuf,
        pakchunk_dir_path: PathBuf
    }
    
    #[before_all]
    fn setup() -> (Arc<Context>, ()){
        let (tmp_fixtures_dir, tmp_fixtures_dir_path) = crate::tests::make_temp_fixtures(Some("repak"));
        
        (Arc::new(Context { 
            _fixtures_dir: tmp_fixtures_dir,
            fixtures_dir_path: tmp_fixtures_dir_path.clone(),
            pakchunk_path: tmp_fixtures_dir_path.join("pakchunk.pak"),
            pakchunk_dir_path: tmp_fixtures_dir_path.join("pakchunk")
        }), ())
    }
    
    #[test]
    fn can_list_pak_file(ctx: Arc<Context>) {
        let mut expected: Vec<(String, PathBuf)> = vec![
            (String::from("8abc079520c2e28c7385e34ac1cd1e4e8fb31056"), "RED/Content/Localization/INT/REDGame.uasset".into()),
            (String::from("c7324db8d442f9607b9118550976da84e080e204"), "RED/Content/Chara/FAU/Common/Data/BBS_FAUEF.uasset".into()),
            (String::from("af3c7f6d712c520ba490a8fe49e3c2b99b982ef7"), "RED/Content/Chara/FAU/Common/Data/BBS_FAU.uasset".into()),
            (String::from("d31b00f7283e6d10fba36d85fc6d230a84141c8b"), "RED/Content/Chara/FAU/Common/Data/BBS_FAU_BOSS.uasset".into()),
            (String::from("5baff11588777b63891d32d49ddc4c411bf107be"), "RED/Content/Chara/FAU/Common/Data/409/BBS_FAUEF.uasset".into()),
            (String::from("a95e526c1a11c697fab26e4c470e45a3f662978d"), "RED/Content/Chara/FAU/Common/Data/409/BBS_FAU.uasset".into()),
            (String::from("11d332b18aeeca51b26c964f1eea8f612404cd79"), "RED/Content/Chara/FAU/Common/Data/409/BBS_FAU_BOSS.uasset".into()),
            (String::from("64a5fff0949345c106393f40aebc58e28fd105cb"), "RED/Content/Chara/FAU/Common/Data/409/COL_FAU.uasset".into()),
            (String::from("c5f428058f13334eb380e50ed549a61b1ffb710f"), "RED/Content/Chara/FAU/Common/Data/409/BBS_FAUEF_BOSS.uasset".into()),
            (String::from("c118d88db7356cd9d1e1309814bc6c84cdd584c4"), "RED/Content/Chara/FAU/Common/Data/BBS_FAUEF_BOSS.uasset".into()),
            (String::from("4f471984360146fd4d1f4dd0be8368774ee52372"), "RED/Content/Chara/FAU/Common/Data/COL_FAU.uasset".into()),
            (String::from("130479f6d5d56fa1d32a58d80371f888541f6917"), "RED/Content/Chara/FAU/Common/Data/BBS_FAUEF.uexp".into()),
            (String::from("b637fc816934b0fbc59de9c0c06d202003a8be69"), "RED/Content/Chara/FAU/Common/Data/409/BBS_FAUEF.uexp".into()),
            (String::from("4ba11921843f01d9ddf8a076f87d50e09d6c373e"), "RED/Content/Chara/FAU/Common/Data/BBS_FAU_BOSS.uexp".into()),
            (String::from("ecec9c7adcd6f8be9d8002652763ccb7cb94e89a"), "RED/Content/Chara/FAU/Common/Data/409/BBS_FAUEF_BOSS.uexp".into()),
            (String::from("c16fec016fa054a0c3a64f95cd110af809355408"), "RED/Content/Chara/FAU/Common/Data/BBS_FAUEF_BOSS.uexp".into()),
            (String::from("e83c6faec591e041b7bd2e8a17b9d44a03a643ff"), "RED/Content/Chara/FAU/Common/Data/409/BBS_FAU.uexp".into()),
            (String::from("35ab98ed0d4b3fc90563b777e70e3ae836ed2281"), "RED/Content/Chara/FAU/Common/Data/409/BBS_FAU_BOSS.uexp".into()),
            (String::from("018b6e60e3f01bdefe2a3b512e9248dec1cd8e1c"), "RED/Content/Chara/FAU/Common/Data/BBS_FAU.uexp".into()),
            (String::from("991d9d84143d561f5c46bb035f56ced4863b6191"), "RED/Content/Chara/FAU/Common/Data/COL_FAU.uexp".into()),
            (String::from("e07fac2bce905e21fc74322cd7c6c4e7b149b601"), "RED/Content/Chara/FAU/Common/Data/409/COL_FAU.uexp".into()),
            (String::from("d988c554f720aa7bbb7e8d832db52fcbbd5a2cfb"), "RED/Content/Localization/INT/REDGame.uexp".into()),
        ];
        let reader = super::PakReader::new(
            &ctx.pakchunk_path, 
            crate::TargetGame::GGST.aes_key()
        ).unwrap();
        
        let mut list = reader.list(None).unwrap().into_iter()
            .map(|pf| (hex::encode(pf.hash), pf.path.to_path_buf()))
            .collect::<Vec<(String, PathBuf)>>();
        expected.sort();
        list.sort();
        
        assert_eq!(expected, list);
    }
    
    
    #[test]
    fn validate_succeeds_when_pak_content_matches_dir_content(ctx: Arc<Context>) {
        let result = super::validate(
            &ctx.pakchunk_path, 
            crate::TargetGame::GGST.aes_key(), 
            &ctx.pakchunk_dir_path,
            None
        );
        assert!(result);
    }
    
    #[test]
    fn validate_succeeds_when_pak_content_matches_dir_content_filtered(ctx: Arc<Context>) {
        let filtered_pakchunk_dir_path = ctx.fixtures_dir_path.join("pakchunk-filtered");
        let result = super::validate(
            &ctx.pakchunk_path, 
            crate::TargetGame::GGST.aes_key(), 
            &filtered_pakchunk_dir_path,
            Some(&[
                "**/Localization/**/*.uasset", 
                "**/COL*.uexp"
            ])
        );
        assert!(result);
    }
    
    #[test]
    fn validate_fails_when_pak_content_doesnt_match_dir_content(ctx: Arc<Context>) {
        let foo_file = ctx.pakchunk_dir_path.join("foo.bar");
        std::fs::write(&foo_file, b"foo_bar").unwrap();
        
        let result = super::validate(
            &ctx.pakchunk_path, 
            crate::TargetGame::GGST.aes_key(), 
            &ctx.pakchunk_dir_path,
            None
        );
        assert!(!result);
        
        let _ = std::fs::remove_file(&foo_file);
    }
    
    #[test]
    fn can_unpack_pak(ctx: Arc<Context>) {
        let pakchunk_dir_path = ctx.fixtures_dir_path.join("pakchunk");
        let out_dir = ctx.fixtures_dir_path.join("extracted");
        let result = super::unpack(&ctx.pakchunk_path, crate::TargetGame::GGST.aes_key(), &out_dir, 
            None, |_, _| {}, |_| {}
        );
        result.unwrap();
        
        assert!(!dir_diff::is_different(&pakchunk_dir_path, &out_dir).unwrap());   
        
        let _ = std::fs::remove_dir_all(out_dir);
    }
    
    #[test]
    fn can_unpack_specific_paths_inside_pak(ctx: Arc<Context>) {
        let filtered_pakchunk_dir_path = ctx.fixtures_dir_path.join("pakchunk-filtered");
        let out_dir = ctx.fixtures_dir_path.join("extracted");
        let result = super::unpack(&ctx.pakchunk_path, crate::TargetGame::GGST.aes_key(), &out_dir, 
            Some(&[
                "**/Localization/**/*.uasset", 
                "**/COL*.uexp"
            ]),
            |_, _| {}, |_| {}
        );
        result.unwrap();
        
        assert!(!dir_diff::is_different(&filtered_pakchunk_dir_path, &out_dir).unwrap());   
        
        let _ = std::fs::remove_dir_all(out_dir);
    }
    
    #[test]
    fn can_pack_directory(ctx: Arc<Context>) {
        let packed_path = ctx.fixtures_dir_path.join("pack_dir_test.pak");
        let result = super::pack(
            crate::TargetGame::GGST.aes_key(),
            super::Version::V3,
            "../../..",
            &ctx.fixtures_dir_path.join("pakchunk"),
            &packed_path
        );
        result.unwrap();
        
        assert_eq!(sha1_hash_file(&packed_path).ok(), sha1_hash_file(&ctx.pakchunk_path).ok());
        let _ = std::fs::remove_file(packed_path);
    }
    
    #[test]
    fn can_read_file_inside_pak(ctx: Arc<Context>) {
        let rel_file_path = "RED/Content/Chara/FAU/Common/Data/BBS_FAU.uexp";
        let ref_file_path = ctx.fixtures_dir_path.join("pakchunk").join(rel_file_path);
        let ref_file_contents = std::fs::read(ref_file_path).unwrap();
        
        let pak_file_contents = super::read_file(
            &ctx.pakchunk_path,
            crate::TargetGame::GGST.aes_key(), 
            rel_file_path
        ).unwrap();
        
        assert_eq!(ref_file_contents, pak_file_contents);
    }
}

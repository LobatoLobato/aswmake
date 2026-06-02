use std::{fs::File, io::BufReader, path::PathBuf, str::FromStr};

use aes::cipher::{KeyInit as _};
use itertools::Itertools;
use rayon::iter::{IntoParallelRefIterator as _, ParallelBridge as _, ParallelIterator};
use glob_match::glob_match;

use crate::{AResult, error, path::Path, util::HashId};

use strum::{EnumString, Display};
use derivative::Derivative;

pub use repak::Version;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, EnumString, Ord, Display, Hash)]
#[strum(serialize_all = "lowercase")]
pub enum PakFileKind {
    Uasset,
    Uexp,
    Other
}

#[derive(Derivative)]
#[derivative(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PakFile<'a> {
    #[derivative(Debug="ignore", PartialEq="ignore", PartialOrd="ignore", Ord="ignore")]
    reader: &'a PakReader,
    pub path: &'a std::path::Path,
    pub kind: PakFileKind,
    pub size: u64,
    pub hash: HashId,
}

impl<'a> PakFile <'a> {
    fn new(path: &'a String, size: u64, reader: &'a PakReader) -> PakFile<'a> {
        let kind = if let Some(ext) = path.as_path().extension() {
            PakFileKind::from_str(&ext.to_string_lossy()).unwrap_or(PakFileKind::Other)
        } else {
            PakFileKind::Other
        };

        PakFile {
            reader,
            path: path.as_path(),
            hash: 1,
            size,
            kind
        }
    }
    pub fn query(&mut self) -> AResult<&Self> {
        let buffer = self.reader.read_file(&self.path)?;
        self.hash = crate::util::hashid_from_reader(std::io::Cursor::new(&buffer))?;
        self.size = buffer.len() as u64;
        Ok(self)
    }
}

pub struct PakReader {
    pak: repak::PakReader,
    pak_path: PathBuf,
    aes_key: aes::Aes256
}

pub type SimpleEntry<'a> = (HashId, &'a std::path::Path);

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

    pub fn keep_files(&mut self, filters: &[&str]) {
        self.pak.keep_files(filters);
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


    pub fn list<'a>(&'a self, filters: Option<&[&str]>) -> Vec<PakFile<'a>> {
        self.list_iter(filters).par_bridge().collect()
    }
    pub fn list_simple<'a>(&'a self, filters: Option<&[&str]>) -> AResult<Vec<SimpleEntry<'a>>> {
        self.list_simple_iter(filters).par_bridge().collect()
    }
    pub fn list_simple_iter<'a>(&'a self, filters: Option<&[&str]>) -> impl Iterator<Item = AResult<SimpleEntry<'a>>> {
        self.list_iter(filters).map(|mut pf| {pf.query()?; Ok(pf)}).map(|pf| pf.map(|pf|(pf.hash, pf.path)))
    }
    pub fn list_iter<'a>(&'a self, filters: Option<&[&str]>) -> impl Iterator<Item = PakFile<'a>> {
        let filter = move |(file_path, size): (&'a String, u64)| match filters {
            Some(filters) => filters.iter().any(|filter| glob_match(filter, file_path)).then_some((file_path, size)),
            None => Some((file_path, size))
        };

        self.pak.files_ref_with_size().into_iter()
            .filter_map(move |file_path| filter(file_path))
            .map(move |(path, size)| PakFile::new(path, size, self))
    }

    pub fn read_file(&self, file_path: impl Path) -> AResult<Vec<u8>> {
        let mut buffer = vec![];
        let mut pak_bufreader = Self::create_buf_reader(&self.pak_path)?;
        self.pak.read_file(&&file_path.as_path().to_string_lossy(), &mut pak_bufreader, &mut buffer)?;
        Ok(buffer)
    }

    pub fn file_len(&self, file_path: impl Path) -> Option<u64> {
        self.pak.file_len_p(file_path.as_path())
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
                let hash = crate::util::hashid_from_file(df.path()).unwrap();

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

    use crate::{path::Path, util::{HashId, hashid_from_file}};

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
        let mut expected: Vec<(HashId, &std::path::Path)> = vec![
            (0xe53cf262dc992f8b, "RED/Content/Localization/INT/REDGame.uasset".as_path()),
            (0x7bdc7602c7ee0aaf, "RED/Content/Localization/INT/REDGame.uexp".as_path()),
            (0xd33fcdf0a9c0357c, "RED/Content/Chara/FAU/Common/Data/BBS_FAUEF.uasset".as_path()),
            (0xed81ca66cd1e61d3, "RED/Content/Chara/FAU/Common/Data/BBS_FAU.uasset".as_path()),
            (0xce9fa6e7a5cafdd9, "RED/Content/Chara/FAU/Common/Data/BBS_FAUEF.uexp".as_path()),
            (0xcc4ccd9328d04dba, "RED/Content/Chara/FAU/Common/Data/BBS_FAU_BOSS.uasset".as_path()),
            (0xba392505e806eb96, "RED/Content/Chara/FAU/Common/Data/COL_FAU.uexp".as_path()),
            (0xc62784251eec7c1f, "RED/Content/Chara/FAU/Common/Data/BBS_FAU_BOSS.uexp".as_path()),
            (0x0574ba95cff7129e, "RED/Content/Chara/FAU/Common/Data/BBS_FAU.uexp".as_path()),
            (0xbb585e32fa7f0109, "RED/Content/Chara/FAU/Common/Data/409/BBS_FAUEF.uasset".as_path()),
            (0x5051152017604e5f, "RED/Content/Chara/FAU/Common/Data/409/BBS_FAU.uasset".as_path()),
            (0xa632dd1e77192f39, "RED/Content/Chara/FAU/Common/Data/409/BBS_FAUEF.uexp".as_path()),
            (0x22c128bbbacbb77c, "RED/Content/Chara/FAU/Common/Data/409/BBS_FAU_BOSS.uasset".as_path()),
            (0x8a53b3ef08e59c72, "RED/Content/Chara/FAU/Common/Data/409/COL_FAU.uexp".as_path()),
            (0x5034b91c85493cd9, "RED/Content/Chara/FAU/Common/Data/409/BBS_FAU_BOSS.uexp".as_path()),
            (0x448ceb1b17cced1d, "RED/Content/Chara/FAU/Common/Data/409/BBS_FAU.uexp".as_path()),
            (0xf537acf7f8164a3c, "RED/Content/Chara/FAU/Common/Data/409/BBS_FAUEF_BOSS.uasset".as_path()),
            (0x2b3b11c9c9a70e5d, "RED/Content/Chara/FAU/Common/Data/409/BBS_FAUEF_BOSS.uexp".as_path()),
            (0xd58c151dbeac3bbc, "RED/Content/Chara/FAU/Common/Data/409/COL_FAU.uasset".as_path()),
            (0xcd1346e12d43386f, "RED/Content/Chara/FAU/Common/Data/BBS_FAUEF_BOSS.uasset".as_path()),
            (0xfdb08e5db0dad355, "RED/Content/Chara/FAU/Common/Data/BBS_FAUEF_BOSS.uexp".as_path()),
            (0x3fa32008409f3299, "RED/Content/Chara/FAU/Common/Data/COL_FAU.uasset".as_path()),
        ];
        let reader = super::PakReader::new(
            &ctx.pakchunk_path,
            crate::TargetGame::GGST.aes_key()
        ).unwrap();

        let mut list = reader.list_simple(None).unwrap();

        expected.sort();
        list.sort();

        assert_eq!(list, expected);
    }

    #[test]
    fn can_list_filtered_pak_file(ctx: Arc<Context>) {
        let mut expected: Vec<(HashId, &std::path::Path)> = vec![
            (0x8a53b3ef08e59c72, "RED/Content/Chara/FAU/Common/Data/409/COL_FAU.uexp".as_path()),
            (0xd58c151dbeac3bbc, "RED/Content/Chara/FAU/Common/Data/409/COL_FAU.uasset".as_path()),
        ];
        let reader = super::PakReader::new(
            &ctx.pakchunk_path,
            crate::TargetGame::GGST.aes_key()
        ).unwrap();

        let mut list = reader.list_simple(Some(&["**/409/COL_*"])).unwrap();

        expected.sort();
        list.sort();

        assert_eq!(list, expected);
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

        assert_eq!(hashid_from_file(&packed_path).ok(), hashid_from_file(&ctx.pakchunk_path).ok());
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

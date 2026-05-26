use std::{collections::HashMap, io::Cursor, path::PathBuf, sync::Arc};

use aswmake_lib::{AResult, TargetGame, parsers::*, tools::{bbscript, bbspack, repak::{PakFileKind, PakReader}}};
use parking_lot::RwLock;
use crate::m_s::{self, FKind};

use strum::{EnumProperty, VariantArray};
#[derive(Debug, Clone, Copy, EnumProperty, VariantArray)]
#[strum(serialize_all = "lowercase")]
pub enum FileKind {
    #[strum(props(
        path_template = "${parent}/${file_stem}.bbs",
        glob = "**/Content/Chara/**/Data/**/BBS_*",
        compile_glob = "**/Content/Chara/**/Data/**/BBS_*.bbs",
    ))]
    BBS,
    #[strum(props(
        path_template = "${parent}/move_list.json",
        glob = "**/Content/Chara/**/Data/BBS_[!_][!_][!_].uexp",
    ))]
    MoveList,
    #[strum(props(
        path_template = "${parent}/${file_stem}.pac",
        glob = "**/Content/Chara/**/Data/**/COL_*",
        compile_glob = "**/Content/Chara/**/Data/**/COL_*.pac"
    ))]
    PAC,
    #[strum(props(
        path_template = "${parent}/${file_stem}.loc",
        glob = "**/Content/Localization/**/*.uexp",
        compile_glob = "**/Content/Localization/**/*.loc"
    ))]
    LOC,
    #[strum(props(
        no_processing = true,
        glob = "**/Content/**/*.mp4",
        compile_glob = "**/Content/**/*.mp4"
    ))]
    MP4,
    // #[strum(props(
    //     path_template = "${parent}/${file_stem}.ogg",
    //     glob = "**/Content/**/Audio/**/*",
    //     compile_glob = "**/Content/**/Audio/**/*.ogg"
    // ))]
    // OGG
}
impl FKind for FileKind {
    fn variants() -> &'static [Self] {
        Self::VARIANTS
    }
    
    fn needs_processing(&self) -> bool {
        self.get_bool("no_processing").is_none_or(|v| !v)
    }
    fn glob_match(&self, path: &str) -> bool {
        glob_match::glob_match(self.get_str("glob").unwrap(), path)
    }
    fn glob(&self) -> &'static str {
        self.get_str("glob").unwrap()
    }
    
    fn compile_glob_match(&self, path: &str) -> bool {
        self.compile_glob().map_or(false, |glob| glob_match::glob_match(glob, path))
    }
    fn compile_glob(&self) -> Option<&'static str> {
        self.get_str("glob")
    }

    fn path_template(&self) -> &str {
        self.get_str("path_template").unwrap()
    }

}

pub struct Context<'a> {
    loc: Arc<RwLock<Option<loc::Loc>>>,
    bbs_map: Arc<RwLock<HashMap<&'a std::path::Path, bbs::BBS>>>
}

impl<'a> Context<'a> {
    const LOC_FILE_PATH: &'static str = "RED/Content/Localization/INT/REDGame.uexp";
    
    fn new() -> Self {
        Self {
            loc: Arc::new(RwLock::new(None)),
            bbs_map: Arc::new(RwLock::new(Default::default()))
        }
    }
}
impl<'a: 'static> m_s::Context<'a> for Context<'a> {
    type Fk = FileKind;
    
    fn target_game(&self) -> TargetGame { TargetGame::GGST }
    
    fn on_before_init(&self, reader: &'a PakReader) -> AResult<()> {
        let loc_uexp = reader.read_file(Self::LOC_FILE_PATH)?;
        self.loc.write().replace(loc::parse_bytes(Cursor::new(loc_uexp))?);
        
        Ok(())
    }
    
    fn on_match(&self, reader: &'a PakReader, kind: &FileKind, paths: &m_s::EntryKindOriginPathMap<'a>) -> AResult<()> {
        match kind {
            FileKind::MoveList => {
                if let Some((uexp_path, _)) = paths.get(&PakFileKind::Uexp) {
                    let uexp = reader.read_file(uexp_path)?;
                    let bbs = bbs::BBS::parse_bytes(Cursor::new(uexp), TargetGame::GGST, self.loc.read().as_ref())?;
                    self.bbs_map.write().insert(uexp_path, bbs);
                }
                Ok(())
            },
            _ => { Ok(())}
        }
    }
    
    fn on_fs_initialized(&mut self, _: &PakReader) -> aswmake_lib::AResult<()> {
        Ok(())
    }

    fn try_parse(&self, reader: &PakReader, kind: &FileKind, paths: &m_s::EntryKindOriginPathMap) -> AResult<Vec<u8>> {
        match kind {
            FileKind::BBS => Self::parse_bbs(reader, paths, TargetGame::GGST),
            FileKind::MoveList => Self::generate_movelist(&self.bbs_map.read(), paths),
            FileKind::PAC => Self::parse_pac(reader, paths),
            FileKind::LOC => Self::parse_loc(reader, paths),
            // FileKind::OGG => Self::parse_audio(reader, paths),
            _ => { Err(anyhow::anyhow!("File kind {kind:?} is not parsable")) }
        }
    }

    fn on_query_size(&self, reader: &'a PakReader, kind: &FileKind, paths: &m_s::EntryKindOriginPathMap<'a>) -> AResult<Option<u64>> {
        match kind {
            FileKind::LOC => {
                if let Some((_, uexp_len)) = paths.get(&PakFileKind::Uexp) {
                    return Ok(Some(bbspack::extracted_len(*uexp_len as usize) as u64));
                }
                Ok(None)
            },
            FileKind::MoveList => {
                if let Some((uexp_path, _)) = paths.get(&PakFileKind::Uexp) {
                    let uexp = reader.read_file(uexp_path)?;
                    let bbs = bbs::BBS::parse_bytes(Cursor::new(uexp), TargetGame::GGST, self.loc.read().as_ref())?;
                    return Ok(Some(bbs.bytes_len() as u64));
                }
                Ok(None)
            },
            FileKind::BBS => {
                if let Some((uexp_path, _)) = paths.get(&PakFileKind::Uexp) {
                    let uexp_bytes = reader.read_file(uexp_path)?;
                    let extracted_bytes = bbspack::extract_bytes(Cursor::new(uexp_bytes))?;
                    let size = aswmake_lib::bbscript::dry_parse_bytes(
                        self.target_game().to_supported_game(),
                        &mut extracted_bytes.as_slice(),
                        None, None,
                        false,
                        12
                    )?;
                    
                    return Ok(Some(size as u64));
                }
                Ok(None)
            },
            FileKind::PAC => {
                if let Some((_, uexp_len)) = paths.get(&PakFileKind::Uexp) {
                    return Ok(Some(bbspack::extracted_len(*uexp_len as usize) as u64));
                }
                Ok(None)
            },
            // FileKind::OGG => {
            //     if let Some((uexp_path, _)) = paths.get(&PakFileKind::Uexp) {
            //         let uexp_bytes = reader.read_file(uexp_path)?;
            //         Ok(ogg::extract_bytes(uexp_bytes).len());
            //     }
            //     Ok(None)
            // },
            _ => {Ok(None)}
        }
    }

    fn try_compile(
        &self, 
        input_bytes: Vec<u8>, 
        input_rel_path: &std::path::Path, 
        reader: &PakReader, 
        kind: &FileKind
    ) -> AResult<Vec<(PathBuf, Vec<u8>)>> {
        let read_file = |path: PathBuf| -> AResult<(PathBuf, Vec<u8>)> {
            Ok((path.clone(), reader.read_file(path)?, ))
        };
        match kind {
            FileKind::BBS => {
                let mut uasset = read_file(input_rel_path.with_extension("uasset"))?;
                let mut uexp = read_file(input_rel_path.with_extension("uexp"))?;
                
                let rebuilt_script = bbscript::rebuild_bytes(Cursor::new(input_bytes), self.target_game())?;
                bbspack::inject_bytes(&rebuilt_script, &mut uexp.1, &mut uasset.1)?;
                
                Ok(vec![uasset, uexp])
            },
            FileKind::PAC => {
                let mut uasset = read_file(input_rel_path.with_extension("uasset"))?;
                let mut uexp = read_file(input_rel_path.with_extension("uexp"))?;
                
                bbspack::inject_bytes(&input_bytes, &mut uexp.1, &mut uasset.1)?;
                
                Ok(vec![uasset, uexp])
            },
            FileKind::LOC => {
                let mut uasset = read_file(input_rel_path.with_extension("uasset"))?;
                let mut uexp = read_file(input_rel_path.with_extension("uexp"))?;
                
                bbspack::inject_bytes(&input_bytes, &mut uexp.1, &mut uasset.1)?;
                
                Ok(vec![uasset, uexp])
            },
            FileKind::MP4 => {
                Ok(vec![(input_rel_path.to_path_buf(), input_bytes)])
            },
            // FileKind::OGG => {
            //     Ok(vec![])
            // }
            _ => {Ok(vec![])}
        }
        // Ok(())
    }


}



impl<'a> Context<'a> {
    fn parse_bbs(reader: &PakReader, paths: &m_s::EntryKindOriginPathMap, tg: TargetGame) -> AResult<Vec<u8>> {
        let uexp_bytes = reader.read_file(paths.get(&PakFileKind::Uexp).ok_or(anyhow::anyhow!("No uexp for bbs"))?.0)?;
        let extracted_bytes = bbspack::extract_bytes(Cursor::new(uexp_bytes))?;
        bbscript::parse_bytes(&mut extracted_bytes.as_slice(), tg)
    }
    
    fn parse_pac(reader: &PakReader, paths: &m_s::EntryKindOriginPathMap) -> AResult<Vec<u8>> {
        let uexp_bytes = reader.read_file(paths.get(&PakFileKind::Uexp).ok_or(anyhow::anyhow!("No uexp for pac"))?.0)?;
        bbspack::extract_bytes(Cursor::new(uexp_bytes))
    }
    
    fn parse_loc(reader: &PakReader, paths: &m_s::EntryKindOriginPathMap) -> AResult<Vec<u8>> {
        let uexp_bytes = reader.read_file(paths.get(&PakFileKind::Uexp).ok_or(anyhow::anyhow!("No uexp for loc"))?.0)?;
        bbspack::extract_bytes(Cursor::new(uexp_bytes))
    }
    
    // fn parse_ogg(reader: &PakReader, paths: &m_s::EntryKindOriginPathMap) -> AResult<Vec<u8>> {
    //     Ok(vec![])
    // }
    fn generate_movelist(bbs_map: &HashMap<&std::path::Path, bbs::BBS>, paths: &m_s::EntryKindOriginPathMap) -> AResult<Vec<u8>> {
        let key = paths.get(&PakFileKind::Uexp).ok_or(anyhow::anyhow!("No uexp for move_list"))?.0;
        if let Some(bbs) = bbs_map.get(key) {
            Ok(bbs.render().into_bytes())
        } else {
            Err(anyhow::anyhow!("Key {} not found in movelist map", key.display()))
        }
    }
}

pub fn new<'a>() -> Context<'a> {
    Context::new()
}
use std::{io::Cursor, sync::Arc};

use anyhow::anyhow;
use aswmake_lib::{AResult, TargetGame, assets::{self, Asset, CompilationOutput}, parsers::*, tools::repak::PakReader};
use parking_lot::RwLock;


pub struct Context {
    loc: Arc<RwLock<Option<loc_map::LocMap>>>,
}

impl<'a>  Context {
    const LOC_FILE_PATH: &'static str = "RED/Content/Localization/INT/REDGame.uexp";

    fn new() -> Self {
        Self {
            loc: Arc::new(RwLock::new(None))
        }
    }

    pub fn target_game(&self) -> TargetGame { TargetGame::GGST }

    pub fn on_before_init(&self, reader: &'a PakReader) -> AResult<()> {
        let loc_uexp = reader.read_file(Self::LOC_FILE_PATH)?;
        self.loc.write().replace(loc_map::parse_bytes(Cursor::new(loc_uexp))?);

        Ok(())
    }

    pub fn on_match(&self, _reader: &'a PakReader, _asset: &Asset) -> AResult<()> {
        Ok(())
    }

    pub fn on_query_size(&self, reader: &'a PakReader, asset: &Asset) -> AResult<Option<u64>> {
        match asset.kind() {
            assets::movelist::Kind => asset.query_size(reader, self.loc.read().as_ref().map(|l| l as &dyn std::any::Any)),
            _ => asset.query_size(reader, None)
        }
    }

    pub fn on_fs_initialized(&mut self, _: &PakReader) -> aswmake_lib::AResult<()> {
        Ok(())
    }

    pub fn try_parse(&self, reader: &PakReader, asset: &Asset) -> AResult<Vec<u8>> {
        match asset.kind() {
            assets::movelist::Kind => asset.parse(reader, self.loc.read().as_ref().map(|l| l as &dyn std::any::Any)),
            _ => asset.parse(reader, None)
        }
    }

    pub fn try_compile(&self, input_bytes: Vec<u8>, reader: &PakReader, asset: &Asset) -> AResult<CompilationOutput> {
        asset.compile(reader, &input_bytes).and_then(|o| o.ok_or(anyhow!("Asset cant be compiled")))
    }
}

pub fn new<'a>() -> Context {
    Context::new()
}

use std::{io::Cursor, path::PathBuf};

use crate::{AResult, TargetGame, parsers::loc_map::LocMap, tools::repak::PakReader};
use super::{Asset, CompilationOutput};
use debug_log::debug_dbg;
declare_asset! {
    kind: "movelist",
    glob: "**/Content/Chara/**/Data/BBS_[!_][!_][!_].uexp",
    compile_glob: None,
    path_template: "${parent}/move_list.json",
    no_processing: false,

    fn recover_parsed_path(_: &super::AssetCtor, parsed_path: &std::path::Path, _: TargetGame) -> PathBuf {
        parsed_path.to_path_buf()
    }

    fn parse_fn(asset: &Asset, reader: &PakReader, loc: Option<&dyn std::any::Any>) -> AResult<Vec<u8>> {
        debug_dbg!("Parsing movelist with rel_path {} for {:?}", asset.rel_path_noext.display(), asset.target_game);
        let loc_map = loc.and_then(|loc| loc.downcast_ref::<LocMap>());
        let bytes = reader.read_file(&asset.rel_path_noext.with_extension("uexp"))?;
        let mvlist = crate::parsers::movelist::MoveList::parse_bytes(Cursor::new(bytes), asset.target_game, loc_map)?;
        Ok(mvlist.render().into_bytes())
    }

    fn compile_fn(asset: &Asset, _: &PakReader, _: &Vec<u8>) -> AResult<Option<CompilationOutput>> {
        debug_dbg!("Compiling movelist with rel_path {} for {:?}", asset.rel_path_noext.display(), asset.target_game);
        Ok(None)
    }

    fn query_size_fn(asset: &Asset, reader: &PakReader, loc: Option<&dyn std::any::Any>) -> AResult<Option<u64>>{
        debug_dbg!("Querying movelist size with rel_path {} for {:?}", asset.rel_path_noext.display(), asset.target_game);
        let loc_map = loc.and_then(|loc| loc.downcast_ref::<LocMap>());
        let bytes = reader.read_file(&asset.rel_path_noext.with_extension("uexp"))?;
        let mvlist = crate::parsers::movelist::MoveList::parse_bytes(Cursor::new(bytes), asset.target_game, loc_map)?;
        Ok(Some(mvlist.render().as_bytes().len() as u64))
    }
}

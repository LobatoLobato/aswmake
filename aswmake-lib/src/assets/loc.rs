use std::{io::Cursor, path::PathBuf};

use crate::{AResult, TargetGame, tools::{self, repak::PakReader}};
use super::{Asset, CompilationOutput};
use debug_log::debug_dbg;

declare_asset! {
    kind: "localization file",
    glob: "**/Content/Localization/**/*.uexp",
    compile_glob: Some("**/Content/Localization/**/*.loc"),
    path_template: "${parent}/${file_stem}.loc",
    no_processing: false,

    fn recover_parsed_path(_: &super::AssetCtor, parsed_path: &std::path::Path, _: TargetGame) -> PathBuf {
        parsed_path.with_extension("")
    }

    fn parse_fn(asset: &Asset, reader: &PakReader, _: Option<&dyn std::any::Any>) -> AResult<Vec<u8>> {
        debug_dbg!("Parsing loc with rel_path {} for {:?}", asset.rel_path_noext.display(), asset.target_game);
        let bytes = reader.read_file(asset.rel_path_noext.with_extension("uexp"))?;
        tools::bbspack::extract_bytes(Cursor::new(bytes))
    }

    fn compile_fn(asset: &Asset, reader: &PakReader, input: &Vec<u8>) -> AResult<Option<CompilationOutput>> {
        debug_dbg!("Compiling loc with rel_path {} for {:?}", asset.rel_path_noext.display(), asset.target_game);
        let mut uasset = super::read_file(reader, asset.rel_path_noext.with_extension("uasset"))?;
        let mut uexp = super::read_file(reader, asset.rel_path_noext.with_extension("uexp"))?;

        let output = tools::bbspack::inject_bytes(input, &mut uexp.1, &mut uasset.1)?;

        Ok(Some((vec![uasset, uexp], output)))
    }

    fn query_size_fn(asset: &Asset, reader: &PakReader, _: Option<&dyn std::any::Any>) -> AResult<Option<u64>> {
        debug_dbg!("Querying loc size with rel_path {} for {:?}", asset.rel_path_noext.display(), asset.target_game);
        let uexp_len = reader.file_len(asset.rel_path_noext.with_extension("uexp"));
        let size = uexp_len.map(|l| tools::bbspack::extracted_len(l as usize) as u64);
        return Ok(size);
    }
}

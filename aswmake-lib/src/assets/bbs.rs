use std::{io::Cursor, path::PathBuf};

use crate::{AResult, TargetGame, assets::{Asset, CompilationOutput}, tools::{self, repak::PakReader}};
use debug_log::debug_dbg;

declare_asset! {
    kind: "bbscript",
    glob: "**/Content/Chara/**/Data/**/BBS_*",
    compile_glob: Some("**/Content/Chara/**/Data/**/BBS_*.bbs"),
    path_template: "${parent}/${file_stem}.bbs",
    no_processing: false,

    fn recover_parsed_path(_: &super::AssetCtor, parsed_path: &std::path::Path, _: TargetGame) -> PathBuf {
        parsed_path.with_extension("")
    }

    fn query_size_fn(asset: &Asset, reader: &PakReader, _: Option<&dyn std::any::Any>) -> AResult<Option<u64>> {
        debug_dbg!("Querying bbs size with rel_path {} for {:?}", asset.rel_path_noext.display(), asset.target_game);
        let uexp_bytes = reader.read_file(asset.rel_path_noext.with_extension("uexp"))?;
        let extracted_bytes = tools::bbspack::extract_bytes(Cursor::new(uexp_bytes))?;
        let size = crate::bbscript::dry_parse_bytes(
            asset.target_game.to_supported_game(),
            &mut extracted_bytes.as_slice(),
            None, None,
            false,
            12
        )?;

        return Ok(Some(size as u64));
    }

    fn parse_fn(asset: &Asset, reader: &PakReader, _: Option<&dyn std::any::Any>) -> AResult<Vec<u8>> {
        debug_dbg!("Parsing bbs with rel_path {} for {:?}", asset.rel_path_noext.display(), asset.target_game);
        let bytes = reader.read_file(asset.rel_path_noext.with_extension("uexp"))?;
        let extracted_bytes = tools::bbspack::extract_bytes(Cursor::new(bytes))?;
        tools::bbscript::parse_bytes(&mut extracted_bytes.as_slice(), asset.target_game)
    }

    fn compile_fn(asset: &Asset, reader: &PakReader, input: &Vec<u8>) -> AResult<Option<CompilationOutput>> {
        debug_dbg!("Compiling bbs with rel_path {} for {:?}", asset.rel_path_noext.display(), asset.target_game);
        let mut uasset = super::read_file(reader, asset.rel_path_noext.with_extension("uasset"))?;
        let mut uexp = super::read_file(reader, asset.rel_path_noext.with_extension("uexp"))?;

        let rebuilt_script = tools::bbscript::rebuild_bytes(std::io::Cursor::new(input), asset.target_game)?;
        let output = tools::bbspack::inject_bytes(&rebuilt_script, &mut uexp.1, &mut uasset.1)?;

        Ok(Some((vec![uasset, uexp], output)))
    }
}

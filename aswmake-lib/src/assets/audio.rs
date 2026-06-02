use std::path::PathBuf;

use crate::{AResult, TargetGame, tools::{self, repak::PakReader}};
use super::{Asset, CompilationOutput};
use debug_log::debug_dbg;

declare_asset! {
    kind: "audio file",
    glob: "**/Content/**/Audio/**/*",
    compile_glob: Some("**/Content/**/Audio/**/*.ogg"),
    path_template: "${parent}/${file_stem}.ogg",
    no_processing: false,

    fn recover_parsed_path(_: &super::AssetCtor, parsed_path: &std::path::Path, _: TargetGame) -> PathBuf {
        parsed_path.with_extension("")
    }

    fn parse_fn(asset: &Asset, reader: &PakReader, _: Option<&dyn std::any::Any>) -> AResult<Vec<u8>> {
        debug_dbg!("Parsing audio with rel_path {} for {:?}", asset.rel_path_noext.display(), asset.target_game);
        let buffer = reader.read_file(asset.rel_path_noext.with_extension("uexp"))?;

        let ogg_magic = b"OggS";

        if let Some(offset) = buffer.windows(4).position(|window| window == ogg_magic) {
            println!("Cabeçalho OggS encontrado no byte (offset): {}", offset);
            println!("Arquivo .ogg extraído com sucesso!");
            Ok(buffer[offset..].to_vec())
        } else {
            Ok(vec![])
            // Err(anyhow::anyhow!("Assinatura mágica 'OggS' não foi encontrada no arquivo .uexp."))
        }
    }

    fn compile_fn(asset: &Asset, reader: &PakReader, input: &Vec<u8>) -> AResult<Option<CompilationOutput>> {
        debug_dbg!("Compiling pac with rel_path {} for {:?}", asset.rel_path_noext.display(), asset.target_game);
        let mut uasset = super::read_file(reader, asset.rel_path_noext.with_extension("uasset"))?;
        let mut uexp = super::read_file(reader, asset.rel_path_noext.with_extension("uexp"))?;

        let output = tools::bbspack::inject_bytes(&input, &mut uexp.1, &mut uasset.1)?;

        Ok(Some((vec![uasset, uexp], output )))
    }

    fn query_size_fn(asset: &Asset, reader: &PakReader, _: Option<&dyn std::any::Any>) -> AResult<Option<u64>> {
        let buffer = reader.read_file(asset.rel_path_noext.with_extension("uexp"))?;

        let ogg_magic = b"OggS";

        if let Some(offset) = buffer.windows(4).position(|window| window == ogg_magic) {
            println!("Cabeçalho OggS encontrado no byte (offset): {}", offset);
            println!("Arquivo .ogg extraído com sucesso!");
            Ok(Some(buffer[offset..].len() as u64))
        } else {
            Ok(None)
            // Err(anyhow::anyhow!("Assinatura mágica 'OggS' não foi encontrada no arquivo {}.uexp.", asset.rel_path_noext.display()))
        }
    }
}

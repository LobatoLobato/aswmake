use crate::{AResult, TargetGame, tools::repak::PakReader};
use super::{Asset, CompilationOutput};

#[allow(non_upper_case_globals)]
pub const Kind: super::AssetKind = const_fnv1a_hash::fnv1a_hash_str_64("passthrough");

pub fn create(rel_path: &std::path::Path, target_game: TargetGame) -> super::Asset {
    super::Asset {
        kind: Kind,
        parse_fn,
        query_size_fn,
        compile_fn,
        target_game,
        rel_path_noext: rel_path.to_path_buf(),
        no_processing: true
    }
}

pub fn path<'a>(asset: &'a Asset) -> &'a std::path::PathBuf {
    assert!(asset.is_passthrough());
    &asset.rel_path_noext
}
fn parse_fn(asset: &Asset, reader: &PakReader, _: Option<&dyn std::any::Any>) -> AResult<Vec<u8>> {
    reader.read_file(&asset.rel_path_noext)
}

fn compile_fn(_: &Asset, _: &PakReader, _: &Vec<u8>) -> AResult<Option<CompilationOutput>> {
    Ok(None)
}

fn query_size_fn(asset: &Asset, reader: &PakReader, _: Option<&dyn std::any::Any>) -> AResult<Option<u64>>{
    Ok(reader.file_len(&asset.rel_path_noext))

}

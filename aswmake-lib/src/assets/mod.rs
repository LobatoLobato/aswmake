use std::{path::PathBuf, sync::LazyLock};
use crate::{AResult, TargetGame, tools::repak::PakReader};

pub type CompilationOutput = (Vec<(PathBuf, Vec<u8>)>, String);

fn read_file(reader: &PakReader, rel_path: PathBuf) -> AResult<(PathBuf, Vec<u8>)> {
    Ok((rel_path.clone(), reader.read_file(rel_path)?))
}

pub type AssetKind = u64;

#[derive(Debug, Clone)]
pub struct Asset {
    kind: AssetKind,
    parse_fn: fn(asset: &Asset, reader: &PakReader, data: Option<&dyn std::any::Any>) -> AResult<Vec<u8>>,
    query_size_fn: fn(asset: &Asset, reader: &PakReader, data: Option<&dyn std::any::Any>) -> AResult<Option<u64>>,
    compile_fn: fn(asset: &Asset, reader: &PakReader, input: &Vec<u8>) -> AResult<Option<CompilationOutput>>,

    target_game: TargetGame,
    rel_path_noext: PathBuf,

    no_processing: bool
}

impl Asset {
    pub fn kind(&self) -> AssetKind {
        self.kind
    }

    pub fn parse(&self, reader: &PakReader, data: Option<&dyn std::any::Any>) -> AResult<Vec<u8>> {
        (self.parse_fn)(self, reader, data)
    }
    pub fn query_size(&self, reader: &PakReader, data: Option<&dyn std::any::Any>) -> AResult<Option<u64>> {
        (self.query_size_fn)(self, reader, data)
    }
    pub fn compile(&self, reader: &PakReader, input: &Vec<u8>) -> AResult<Option<CompilationOutput>> {
        (self.compile_fn)(self, reader, input)
    }

    pub fn needs_processing(&self) -> bool {
        !self.no_processing
    }

    pub fn is_passthrough(&self) -> bool {
        self.kind == passthrough::Kind
    }
}

pub struct AssetCtor {
    kind: AssetKind,

    glob: &'static str,
    compile_glob: Option<&'static str>,

    path_template: &'static str,
    no_processing: bool,

    create_fn: fn(rel_path: &std::path::Path, target_game: TargetGame) -> Asset,
    recover_parsed_path: fn(ctor: &AssetCtor, parsed_path: &std::path::Path, target_game: TargetGame) -> PathBuf
}


impl AssetCtor {
    pub fn create_asset(&self, rel_path: &std::path::Path, target_game: TargetGame) -> Asset {
        (self.create_fn)(rel_path, target_game)
    }
    pub fn from_parsed(&self, parsed_path: &std::path::Path, target_game: TargetGame) -> Asset {
        let rel_path = (self.recover_parsed_path)(self, parsed_path, target_game);
        (self.create_fn)(rel_path.as_path(), target_game)
    }
    pub fn glob_match(&self, path: &str) -> bool {
        glob_match::glob_match(self.glob, path)
    }
    pub fn compile_glob_match(&self, path: &str) -> bool {
        self.compile_glob.is_some_and(|g| glob_match::glob_match(g, path))
    }

    pub fn path_template(&self) -> &'static str {
        self.path_template
    }

    pub fn needs_processing(&self) -> bool {
        !self.no_processing
    }

    pub fn kind(&self) -> AssetKind {
        self.kind
    }
}

macro_rules! declare_asset {
    {
        kind: $kind:expr,
        glob: $glob:expr,
        compile_glob: $comp_glob:expr,
        path_template: $template:expr,
        no_processing: $no_processing:expr,

        $(fn $fn_name:ident ( $($args:tt)* ) $(-> $ret:ty)? $body:block $(,)?)*
    } => {
        $(fn $fn_name ( $($args)* ) $(-> $ret)? $body)*

        #[allow(non_upper_case_globals)]
        pub const Kind: super::AssetKind = const_fnv1a_hash::fnv1a_hash_str_64($kind);

        fn create_fn(rel_path: &std::path::Path, target_game: TargetGame) -> super::Asset {
            let rel_path_noext = rel_path.with_extension("");

            super::Asset {
                kind: Kind,
                parse_fn,
                query_size_fn,
                compile_fn,
                target_game,
                rel_path_noext,
                no_processing: $no_processing
            }
        }

        pub (super) const CTOR: super::AssetCtor = super::AssetCtor {
            kind: Kind,
            glob: $glob,
            compile_glob: $comp_glob,
            path_template: $template,
            no_processing: $no_processing,
            create_fn,
            recover_parsed_path
        };
    };
}

macro_rules! define_assets {
    ($($name:ident;)*) => {
        $(pub mod $name;)*

        #[allow(non_upper_case_globals)]
        pub static AssetConstructors: LazyLock<Vec<AssetCtor>> = LazyLock::new(|| vec![
            $($name::CTOR,)*
        ]);
    };
}

define_assets! {
    pac;
    bbs;
    loc;
    movelist;
    audio;
}

pub mod sig;
pub mod passthrough;

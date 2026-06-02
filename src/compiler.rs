use std::{path::PathBuf, sync::Arc};

use aswmake_lib::{AResult, assets::AssetConstructors, path::Path, tools::repak::PakReader};
use color_print::cprintln;
use parking_lot::RwLock;

use crate::context::Context;


pub struct Compiler<'a> {
    pak_reader: &'a PakReader,
    context: Arc<RwLock<Context>>
}

impl<'a: 'static> Compiler<'a> {
    pub fn new(
        pak_path: PathBuf,
        context: Context
    ) -> anyhow::Result<Self> {
        use parking_lot::RwLock;
        use PakReader;

        Ok(Self {
            pak_reader: Box::leak(Box::new(PakReader::new(&pak_path, context.target_game().aes_key())?)),
            context: Arc::new(RwLock::new(context))
        })
    }

    pub fn compile(&self, src_dir: impl Path, artifacts_dir: impl Path) -> AResult<()> {
        let src_dir = src_dir.absolute_dir()?;
        let artifacts_dir = artifacts_dir.absolute()?;

        std::fs::create_dir_all(&artifacts_dir)?;

        for file in walkdir::WalkDir::new(&src_dir).into_iter().filter_map(|e| e.ok()).filter(|e| e.file_type().is_file()) {
            let ctor = AssetConstructors.iter().find(|ctor| ctor.compile_glob_match(&file.path().to_string_lossy()));
            let rel_path = file.path().strip_prefix(&src_dir)?;
            if let Some(ctor) = ctor {
                let bytes = std::fs::read(file.path())?;
                let asset = ctor.from_parsed(rel_path, self.context.read().target_game());
                let result = self.context.read().try_compile(bytes, self.pak_reader, &asset)?;

                cprintln!("<blue>-----------------------------------------</blue>");
                cprintln!("<yellow>Compiling {}</yellow>...", rel_path.display());
                result.1.lines().for_each(|l| { cprintln!("<yellow>></yellow><green> {l} </green>"); });

                for (path, bytes) in result.0 {
                    let artifact_path = artifacts_dir.join(path);
                    std::fs::create_dir_all(artifact_path.parent().unwrap())?;
                    std::fs::write(artifact_path, bytes)?;
                }
            } else {

                let artifact_path = artifacts_dir.join(rel_path);
                std::fs::create_dir_all(artifact_path.parent().unwrap())?;
                std::fs::copy(file.path(), artifact_path)?;
            }
        }

        Ok(())
    }

    pub fn package(&self, _artifacts_dir: impl Path, _package_path: impl Path) {
        // aswmake_lib::build::package(cfg.target_game, package_path, compiled_dir, cfg.install_dir.as_ref())?;
    }
}

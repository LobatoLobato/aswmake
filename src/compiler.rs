use std::{path::PathBuf, sync::Arc};

use aswmake_lib::{AResult, path::Path, tools::repak::PakReader};
use parking_lot::RwLock;

use crate::m_s::{Context, FKind};


pub struct Compiler<'a, Ctx: Context<'a>> {
    pak_reader: &'a PakReader,
    context: Arc<RwLock<Ctx>>
}

impl<'a: 'static, Ctx: Context<'a>> Compiler<'a, Ctx> {
    pub fn new(
        pak_path: PathBuf,
        context: Ctx
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
            let fk = Ctx::Fk::variants().iter().find(|fk| fk.compile_glob_match(&file.path().to_string_lossy()));

            if let Some(fk) = fk {
                let path = file.path().strip_prefix(&src_dir)?;
                let bytes = std::fs::read(file.path())?;
                let result = self.context.read().try_compile(bytes, path, self.pak_reader, fk)?;

                for (path, bytes) in result {
                    let artifact_path = artifacts_dir.join(path);
                    std::fs::create_dir_all(artifact_path.parent().unwrap())?;
                    std::fs::write(artifact_path, bytes)?;
                }
            }
        }

        Ok(())
    }

    pub fn package(&self, _artifacts_dir: impl Path, _package_path: impl Path) {

    }
}

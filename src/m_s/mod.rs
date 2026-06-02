use aswmake_lib::assets::{self, Asset};
use aswmake_lib::path::Path;
use aswmake_lib::AResult;


use aswmake_lib::tools::repak::{PakFile, PakFileKind, PakReader};
use itertools::Itertools;
use parking_lot::RwLock;
use rayon::iter::{ParallelBridge, ParallelIterator};
use std::path::PathBuf;
use std::sync::Arc;

use std::collections::HashMap;

mod inode_registry;
use inode_registry::*;

use crate::context::Context;

#[cfg(target_os = "linux")]
mod fuser_fs_impl;
#[cfg(target_os = "windows")]
mod winfsp_fs_impl;

pub type INodeSizeIndex = HashMap<u64, u64>;

pub struct PakFilesystem<'a> {
    pak_reader: &'a PakReader,
    registry: Arc<RwLock<InodeRegistry>>,
    cache: Arc<RwLock<HashMap<INodeNo, Vec<u8>>>>,
    context: Arc<RwLock<Context>>
}

impl<'a: 'static> PakFilesystem<'a> {
    pub fn new(
        pak_path: PathBuf,
        context: Context
    ) -> anyhow::Result<Self> {
        use parking_lot::RwLock;
        use PakReader;

        Ok(Self {
            pak_reader: Box::leak(Box::new(PakReader::new(&pak_path, context.target_game().aes_key())?)),
            registry: Arc::new(RwLock::new(InodeRegistry::new())),
            cache: Arc::new(RwLock::new(HashMap::with_capacity(0))),
            context: Arc::new(RwLock::new(context))
        })
    }

    pub fn generate_size_index(
        &self,
        filters: Option<&[&str]>
    ) -> AResult<INodeSizeIndex>{
        self.context.read().on_before_init(self.pak_reader)?;
        self.populate_registry(filters, None)?;

        let mut index = INodeSizeIndex::new();

        for (ino, entry) in self.registry.read().by_ino.iter() {
            if let InodeEntry{kind: EntryKind::File { asset, size, .. }, ..} = &entry && !asset.is_passthrough() {
                index.insert(ino.0, size.unwrap_or(1));
            }
        }

        Ok(index)
    }

    pub fn init(&mut self, filters: Option<&[&str]>, size_index: Option<INodeSizeIndex>) -> AResult<()> {
        self.context.read().on_before_init(self.pak_reader)?;
        self.populate_registry(filters, size_index)?;
        self.context.write().on_fs_initialized(self.pak_reader)?;
        Ok(())
    }

    fn populate_registry(
        &self,
        filters: Option<&[&str]>,
        size_index: Option<INodeSizeIndex>
    ) -> AResult<()>{
        let grouping_rule = |pf1: &PakFile<'a>, pf2: &PakFile<'a>| {
            let (p1, k1, p2, k2) = (pf1.path, &pf1.kind, pf2.path, &pf2.kind);
            (p1.parent(), p1.file_stem()) == (p2.parent(), p2.file_stem()) &&
            *k1 != PakFileKind::Other && *k2 != PakFileKind::Other
        };
        let sorted_list = self.pak_reader.list_iter(filters).sorted();
        let chunked_list = sorted_list.as_slice().chunk_by(grouping_rule);

        let entries = Arc::new(RwLock::new(vec![]));
        chunked_list.par_bridge().for_each_with(entries.clone(), |entries, files| {
            let any_path = files[0].path;

            let ctor_list = assets::AssetConstructors.iter().filter(|ac| {
                files.iter().any(|f| ac.needs_processing() && ac.glob_match(&f.path.to_string_lossy()))
            }).collect_vec();

            let target_game = self.context.read().target_game();

            for ctor in ctor_list.iter() {
                let file_path = ctor.path_template()
                    .replace("${parent}", &any_path.parent().unwrap().to_string_lossy())
                    .replace("${file_stem}", &any_path.file_stem().unwrap().to_string_lossy())
                    .to_path_buf();

                let asset = ctor.create_asset(any_path, target_game);

                self.context.read().on_match(self.pak_reader, &asset).unwrap();
                let size = if size_index.is_none() {
                    self.context.read().on_query_size(self.pak_reader, &asset).unwrap()
                } else {
                    None
                };

                let entry = EntryKind::File { size, asset };
                entries.write().push((file_path, entry));
            }

            if ctor_list.len() > 0 { return; }
            for file in files {
                let file_path = PathBuf::from(file.path);
                let entry = EntryKind::File {
                    size: Some(file.size),
                    asset: assets::passthrough::create(&file_path, target_game)
                };
                entries.write().push((file_path, entry));
            }
        });

        if let Some(mut entries) = Arc::into_inner(entries).map(RwLock::into_inner) {
            entries.sort_by(|(p1, _), (p2, _)| p1.cmp(&p2));

            let mut next_ino = 2;
            for (file_path, entry) in entries {
                self.registry.write().register_pak_file(&file_path, entry, &mut next_ino);
            }

            if let Some(index) = size_index {
                for (ino, entry) in self.registry.write().by_ino.iter_mut() {
                    if let Some(idx_size) = index.get(&ino.0) && let EntryKind::File{ ref mut size, .. } = entry.kind {
                        *size = Some(*idx_size);
                    }
                }
            }
        }

        Ok(())
    }

    fn parse_file(&self, asset: &Asset) -> anyhow::Result<Vec<u8>> {
        if !asset.is_passthrough() {
            self.context.read().try_parse(&self.pak_reader, asset)
        } else {
            let bytes = self.pak_reader.read_file(assets::passthrough::path(asset))?;
            Ok(bytes)
        }
    }

    fn _generate_file_handle() -> u64{
        static NEXT_HANDLE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

        NEXT_HANDLE.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }
}

use std::{collections::HashMap, path::PathBuf};

use aswmake_lib::tools::repak::{PakFileKind};

#[cfg(target_os = "linux")]
pub use fuser::INodeNo;
#[cfg(target_os = "windows")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct INodeNo(pub u64);

#[derive(Debug, Clone)]
pub enum EntryKind<'a, PK: super::FKind> {
    Directory,
    File { 
        parser_kind: Option<PK>,
        paths: EntryKindOriginPathMap<'a>, 
        size: Option<u64>
    },
}
pub type EntryKindOriginPathMap<'a> = HashMap<PakFileKind, (&'a std::path::Path, u64)>;

impl<'a, PK: super::FKind> EntryKind<'a, PK> {
    #[cfg(target_os = "linux")]
    pub(super) fn as_fuser_filetype(&self) -> fuser::FileType {
        match self {
            EntryKind::Directory => fuser::FileType::Directory,
            EntryKind::File { .. } => fuser::FileType::RegularFile,
        }
    }
    
    pub(super) fn is_dir(&self) -> bool {
        matches!(self, Self::Directory)
    }
    
    pub(super) fn size(&self) -> Option<u64> {
        match self {
            Self::Directory => Some(0),
            Self::File { size, .. } => size.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct InodeEntry<'a, PK: super::FKind> {
    #[cfg(target_os = "linux")]
    pub ino: INodeNo,
    pub name: ustr::Ustr,
    pub kind: EntryKind<'a, PK>,
}

pub struct InodeRegistry<'a, PK: super::FKind> {
    // For lookups via Inode (used by getattr, read, readdir)
    pub by_ino: HashMap<INodeNo, InodeEntry<'a, PK>>,

    // For lookups via Parent Inode + Component Name (used by lookup)
    pub by_hierarchy: HashMap<(INodeNo, ustr::Ustr), INodeNo>, // Maps (parent_ino, name) -> child_ino

    // For tracking folder contents (used by readdir)
    pub children: HashMap<INodeNo, Vec<INodeNo>>, // Maps parent_ino -> Vec<child_inodes>
}

impl<'a, PK: super::FKind> InodeRegistry<'a, PK> {
    pub fn new() -> Self {
        let mut registry = Self {
            by_ino: HashMap::new(),
            by_hierarchy: HashMap::new(),
            children: HashMap::new(),
        };

        registry.by_ino.insert(INodeNo(1), InodeEntry {
            #[cfg(target_os = "linux")]
            ino: INodeNo(1),
            name: Default::default(),
            kind: EntryKind::Directory,
        });

        registry
    }

    pub fn register_pak_file(&mut self, flat_path: &PathBuf, file_kind: EntryKind<'a, PK>, next_ino: &mut u64) {
        let path = flat_path.as_path();
        let mut current_parent_ino = INodeNo(1);

        let components: Vec<_> = path.components().collect();
        let total_components = components.len();

        for (index, component) in components.iter().enumerate() {
            let component_name = ustr::ustr(&component.as_os_str().to_string_lossy());
            let is_last_component = index == total_components - 1;

            let lookup_key = (current_parent_ino, component_name.clone());
            
            if let Some(&existing_ino) = self.by_hierarchy.get(&lookup_key) {
                current_parent_ino = existing_ino;
            } else {
                let assigned_ino = INodeNo(*next_ino);
                *next_ino += 1;
                
                let final_kind = if is_last_component {
                    file_kind.clone()
                } else {
                    EntryKind::Directory
                };

                let entry = InodeEntry {
                    #[cfg(target_os = "linux")]
                    ino: assigned_ino,
                    name: component_name.clone(),
                    kind: final_kind,
                };

                self.by_ino.insert(assigned_ino, entry);
                self.by_hierarchy.insert(lookup_key, assigned_ino);
                self.children.entry(current_parent_ino).or_default().push(assigned_ino);

                current_parent_ino = assigned_ino;
            }
        }
    }
}
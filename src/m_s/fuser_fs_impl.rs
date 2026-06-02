use super::PakFilesystem;

use super::{EntryKind, InodeEntry};

use std::ffi::OsStr;
pub(super) use fuser::{
    Filesystem,
    FileHandle, FileType, FopenFlags, OpenFlags,
    Generation, INodeNo, Errno,
    Request, LockOwner,
    ReplyAttr, ReplyEntry, ReplyDirectory,
    ReplyEmpty, ReplyOpen, ReplyData,
};

impl PakFilesystem<'static> {
    fn make_attribute(ino: INodeNo, kind: &EntryKind) -> fuser::FileAttr {
        let file_type = kind.as_fuser_filetype();
        let size = kind.size().unwrap_or(1);
        fuser::FileAttr {
            ino,
            size,
            blocks: (size + 511) / 512,
            atime: std::time::UNIX_EPOCH,
            mtime: std::time::UNIX_EPOCH,
            ctime: std::time::UNIX_EPOCH,
            crtime: std::time::UNIX_EPOCH,
            kind: file_type,
            perm: if kind.is_dir() { 0o755 } else { 0o644 },
            nlink: if kind.is_dir() { 2 } else { 1 },
            uid: 1000,
            gid: 1000,
            rdev: 0,
            blksize: 512,
            flags: 0,
        }
    }

    pub fn mount(self, mountpoint: impl AsRef<std::path::Path>) -> anyhow::Result<fuser::BackgroundSession> {
        let mut config = fuser::Config::default();
        config.mount_options.push(fuser::MountOption::RO);
        // config.mount_options.push(fuser::MountOption::AutoUnmount);
        config.mount_options.push(fuser::MountOption::FSName("m_s".to_string()));
        config.acl = fuser::SessionACL::RootAndOwner;
        config.n_threads = Some(2);
        config.clone_fd = true;

        std::fs::create_dir_all(&mountpoint).unwrap();

        let session = fuser::spawn_mount2(self, &mountpoint, &config)?;

        Ok(session)
    }
}
impl Filesystem for PakFilesystem<'static> {
    fn getattr(&self, _req: &Request, ino: INodeNo, _fh: Option<FileHandle>, reply: ReplyAttr) {
        let registry = self.registry.read();

        let Some(entry) = registry.by_ino.get(&ino) else {
            return reply.error(Errno::ENOENT);
        };

        let attr = Self::make_attribute(ino, &entry.kind);
        reply.attr(&std::time::Duration::from_secs(1), &attr);
    }

    fn lookup(&self, _req: &Request, parent: INodeNo, name: &OsStr, reply: ReplyEntry) {
        let name_str = match name.to_str() {
            Some(s) => ustr::ustr(s),
            None => return reply.error(Errno::ENOENT)
        };

        let registry = self.registry.read();
        let Some(&child_ino) = registry.by_hierarchy.get(&(parent, name_str)) else {
            return reply.error(Errno::ENOENT);
        };
        let Some(entry) = registry.by_ino.get(&child_ino) else {
            return reply.error(Errno::ENOENT);
        };

        let attr = Self::make_attribute(child_ino, &entry.kind);
        reply.entry(&std::time::Duration::from_secs(0), &attr, Generation(0));
    }

    fn readdir(&self, _req: &Request, ino: INodeNo, _fh: FileHandle, offset: u64, mut reply: ReplyDirectory) {
        if offset == 0 {
            let _ = reply.add(ino, 1, FileType::Directory, ".");
            let _ = reply.add(ino, 2, FileType::Directory, "..");
        }

        let registry = self.registry.read();
        if let Some(child_inodes) = registry.children.get(&ino) {
            let adjusted_offset = if offset > 2 { (offset - 2) as usize } else { 0 };

            for (i, &child_ino) in child_inodes.iter().enumerate().skip(adjusted_offset) {
                let Some(entry) = registry.by_ino.get(&child_ino) else { continue; };

                if reply.add(entry.ino, (i + 3) as u64, entry.kind.as_fuser_filetype(), &entry.name) {
                    break;
                }
            }
        }

        reply.ok();
    }

    fn open(&self, _req: &Request, ino: INodeNo, flags: OpenFlags, reply: ReplyOpen) {
        use nix::fcntl::OFlag;
        let registry = self.registry.read();

        let Some(entry) = registry.by_ino.get(&ino) else {
            return reply.error(Errno::ENOENT);
        };

        println!("Opening {} for request {:?}", entry.name, _req.unique());
        match entry.kind {
            EntryKind::File { .. } => {
                let open_flags = OFlag::from_bits_truncate(flags.0);
                if open_flags.intersects(OFlag::O_WRONLY | OFlag::O_RDWR) {
                    return reply.error(Errno::EROFS);
                }
                // for (kind, (path, size)) in paths {
                //     println!(" with {}: {} [{size}B]", kind, path.display());
                // }

                reply.opened(FileHandle(0), FopenFlags::FOPEN_DIRECT_IO);
            }
            EntryKind::Directory => {
                reply.error(Errno::EISDIR);
            }
        }

    }

    fn read(
        &self,
        _req: &Request,
        ino: INodeNo,
        _fh: FileHandle,
        offset: u64,
        size: u32,
        _flags: OpenFlags,
        _lock_owner: Option<LockOwner>,
        reply: ReplyData,
    ) {
        let registry = self.registry.read();
        if !self.cache.read().contains_key(&ino) {
            let Some(InodeEntry { kind, ..}) = registry.by_ino.get(&ino) else {return reply.error(Errno::ENOENT)};
            let EntryKind::File { asset: asset_kind, .. } = kind else {return reply.error(Errno::ENOENT)};

            if let Ok(content) = self.parse_file(asset_kind) {
                self.cache.write().insert(ino, content);
            } else {
                return reply.error(Errno::EIO);
            }
        }

        if let Some(content) = self.cache.read().get(&ino) {
            println!("{} at offset [{}] requested!", registry.by_ino.get(&ino).unwrap().name, offset);
            let (total_len, start) = (content.len(), offset as usize);
            let end = std::cmp::min(start + size as usize, total_len);

            if start >= total_len { reply.data(&[]) }
            else { reply.data(&content[start..end]) }
        } else {
            return reply.error(Errno::ENOENT);
        }
    }

    fn release(
        &self,
        _req: &Request,
        ino: INodeNo,
        _fh: FileHandle,
        _flags: OpenFlags,
        _lock_owner: Option<LockOwner>,
        _flush: bool,
        reply: ReplyEmpty,
    ) {
        let registry = self.registry.read();

        if let Some(entry) = registry.by_ino.get(&ino) {
            let mut cache = self.cache.write();
            println!("Releasing {}", entry.name);
            cache.remove(&ino);
            cache.shrink_to_fit();
        }

        reply.ok();
    }
}

use super::PakFilesystem;

use windows::Win32::Storage::FileSystem::{
    FILE_APPEND_DATA, FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_READONLY,
    FILE_WRITE_DATA,
};
use winfsp::filesystem::{
    DirMarker, FileInfo, FileSecurity, FileSystemContext, OpenFileInfo, VolumeInfo, WideNameInfo
};
use winfsp::host::{CoarseGuard, FileSystemHost};
use winfsp::service::FileSystemService;
use winfsp::{Result as FspResult, U16CStr};

use super::{
    inode_registry::{EntryKind, INodeNo, InodeRegistry},
};

pub struct PakFileContext {
    ino: INodeNo,
    is_dir: bool,
}

mod error;

pub type BackgroundSession = FileSystemService<FileSystemHost<PakFilesystem<'static>, CoarseGuard>>;
impl PakFilesystem<'static> {
    fn make_file_info(kind: &EntryKind) -> winfsp::filesystem::FileInfo {
        let mut info = winfsp::filesystem::FileInfo::default();
        match kind {
            EntryKind::Directory => {
                info.file_attributes = FILE_ATTRIBUTE_DIRECTORY.0;
            }
            EntryKind::File { size, .. } => {
                let size = size.unwrap_or(1);
                info.file_attributes = FILE_ATTRIBUTE_READONLY.0;
                info.file_size = size;
                info.allocation_size = (size + 511) & !511;
            }
        }

        info
    }

    fn resolve_path(
        registry: &InodeRegistry,
        path: &winfsp::U16CStr,
    ) -> Option<INodeNo> {
        let path_str = path.to_string_lossy();

        if path_str == "\\" || path_str.is_empty() {
            return Some(INodeNo(1));
        }

        let mut current = INodeNo(1);
        for component in path_str.trim_start_matches('\\').split('\\') {
            if component.is_empty() { continue; }

            let name = ustr::ustr(component);
            current = *registry.by_hierarchy.get(&(current, name))?;
        }
        Some(current)
    }

    pub fn mount(self, mount_point: impl AsRef<std::path::Path>) -> FspResult<BackgroundSession> {
        use winfsp::host::{CoarseGuard, FileSystemHost, VolumeParams};

        let init = winfsp::winfsp_init_or_die();
        let mount_point = mount_point.as_ref().to_string_lossy().to_string();

        let fs = std::sync::Mutex::new(Some(self));

        let mut session = winfsp::service::FileSystemServiceBuilder::new()
            .with_start(move || {
                let fs = fs.lock().unwrap().take().ok_or(error::STATUS_INVALID_PARAMETER)?;

                let serial = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .subsec_nanos();

                let mut params = VolumeParams::new();
                params.sector_size(512);
                params.sectors_per_allocation_unit(1);
                params.max_component_length(255);
                params.volume_creation_time(0);
                params.file_info_timeout(10000);
                params.case_sensitive_search(false);
                params.case_preserved_names(true);
                params.unicode_on_disk(true);
                params.persistent_acls(false);
                params.read_only_volume(true);
                params.post_cleanup_when_modified_only(true);
                params.prefix("");
                params.volume_serial_number(serial);
                params.filesystem_name("m_s");

                let mut host: FileSystemHost<_, CoarseGuard> = FileSystemHost::new(params, fs)?;
                host.mount(&mount_point)?;
                host.start()?;
                Ok(host)
            })
            .with_stop(|host| if let Some(h) = host { Ok(h.stop()) } else { Ok(()) })
            .build("m_s", init)?;
        session.start()?;

        Ok(session)
    }
}

impl FileSystemContext for PakFilesystem<'static> {
    type FileContext = PakFileContext;

    fn get_security_by_name(
        &self,
        file_name: &U16CStr,
        _sec: Option<&mut [std::ffi::c_void]>,
        _reparse_point_resolver: impl FnOnce(&U16CStr) -> Option<FileSecurity>,
    ) -> FspResult<FileSecurity> {
        let registry = self.registry.read();
        let ino = Self::resolve_path(&registry, file_name).ok_or(error::STATUS_OBJECT_NAME_NOT_FOUND)?;

        let entry = registry.by_ino.get(&ino).unwrap();
        Ok(FileSecurity {
            attributes: if entry.kind.is_dir() {
                FILE_ATTRIBUTE_DIRECTORY.0
            } else {
                FILE_ATTRIBUTE_READONLY.0
            },
            reparse: false,
            sz_security_descriptor: 0,
        })
    }

    fn open(
        &self,
        file_name: &U16CStr,
        _create_options: u32,
        granted_access: u32,
        file_info: &mut OpenFileInfo,
    ) -> FspResult<Self::FileContext> {
        let registry = self.registry.read();
        let ino = Self::resolve_path(&registry, file_name).ok_or(error::STATUS_OBJECT_NAME_NOT_FOUND)?;

        let entry = registry.by_ino.get(&ino).unwrap();

        if granted_access & (FILE_WRITE_DATA.0 | FILE_APPEND_DATA.0) != 0 {
            return Err(error::STATUS_ACCESS_DENIED);
        }

        *file_info.as_mut() = Self::make_file_info(&entry.kind);

        Ok(PakFileContext {
            ino,
            is_dir: entry.kind.is_dir(),
        })
    }

    fn close(&self, context: Self::FileContext) {
        if !context.is_dir {
            let mut cache = self.cache.write();
            cache.remove(&context.ino);
            cache.shrink_to_fit();
        }
    }

    fn read(&self, context: &Self::FileContext, buffer: &mut [u8], offset: u64) -> FspResult<u32> {
        if context.is_dir {
            return Err(error::STATUS_INVALID_PARAMETER);
        }

        if !self.cache.read().contains_key(&context.ino) {
            let registry = self.registry.read();
            let entry = registry.by_ino.get(&context.ino).ok_or(error::STATUS_OBJECT_NAME_NOT_FOUND)?;
            let EntryKind::File { ref asset, .. } = entry.kind else {
                return Err(error::STATUS_INVALID_PARAMETER);
            };

            let content = self.parse_file(asset).map_err(|_| error::STATUS_IO_DEVICE_ERROR)?;
            self.cache.write().insert(context.ino, content);
        }

        let cache = self.cache.read();
        let content = cache.get(&context.ino).ok_or(error::STATUS_OBJECT_NAME_NOT_FOUND)?;

        let start = offset as usize;
        if start >= content.len() {
            return Ok(0);
        }
        let end = (start + buffer.len()).min(content.len());
        let chunk = &content[start..end];
        buffer[..chunk.len()].copy_from_slice(chunk);
        Ok(chunk.len() as u32)
    }

    fn read_directory(
        &self,
        context: &Self::FileContext,
        _pattern: Option<&U16CStr>,
        marker: DirMarker<'_>,
        buffer: &mut [u8],
    ) -> winfsp::Result<u32> {
        let registry = self.registry.read();
        let children = match registry.children.get(&context.ino) {
            Some(c) => c,
            None => return Ok(0),
        };

        let marker_str = marker.inner_as_cstr().and_then(|m| m.to_string().ok());
        let mut past_marker = marker_str.is_none();
        let mut bytes_written = 0u32;

        for &child_ino in children {
            let Some(entry) = registry.by_ino.get(&child_ino) else {
                continue;
            };
            let name = entry.name.as_str();

            if !past_marker {
                if Some(name) == marker_str.as_deref() {
                    past_marker = true;
                }
                continue;
            }

            let file_info = Self::make_file_info(&entry.kind);
            let wide_name = winfsp::U16CString::from_str(name).unwrap_or_default();

            let mut dir_info = winfsp::filesystem::DirInfo::<255>::new();
            *dir_info.file_info_mut() = file_info;
            dir_info.set_name_cstr(&wide_name)?;

            if !dir_info.append_to_buffer(buffer, &mut bytes_written) {
                break;
            }
        }

        Ok(bytes_written)
    }

    fn get_file_info(&self, ctx: &Self::FileContext, file_info: &mut FileInfo) -> FspResult<()> {
        let registry = self.registry.read();
        let entry = registry.by_ino.get(&ctx.ino).ok_or(error::STATUS_OBJECT_NAME_NOT_FOUND)?;
        *file_info = Self::make_file_info(&entry.kind);
        Ok(())
    }

    fn get_volume_info(&self, volume_info: &mut VolumeInfo) -> FspResult<()> {
        let registry = self.registry.read();
        volume_info.total_size = registry.by_ino.values().map(|e| e.kind.size().unwrap_or(0)).sum();
        volume_info.free_size = 0;
        Ok(())
    }
}

use super::PakFilesystem;

use winfsp::filesystem::{
    FileSystemContext,
    FileInfo, FileSecurity, 
    OpenFileInfo, WideNameInfo, DirMarker
};
use winfsp::{Result as FspResult, FspError, U16CStr};
use windows::Win32::Storage::FileSystem::{
    FILE_APPEND_DATA, FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_READONLY, FILE_FLAG_NO_BUFFERING, FILE_WRITE_DATA
};
use windows::Win32::Foundation::STATUS_ACCESS_DENIED;

use super::{
    Context, FKind,
    inode_registry::{InodeRegistry, EntryKind, INodeNo}
};

pub struct PakFileContext {
    ino: INodeNo,
    is_dir: bool,
}

impl<FK: FKind, Ctx: Context<'static, FK>> PakFilesystem<'static, FK, Ctx> {
    fn make_file_info(kind: &EntryKind<'static, FK>) -> winfsp::filesystem::FileInfo {
        let mut info = winfsp::filesystem::FileInfo::default();
        match kind {
            EntryKind::Directory => {
                info.file_attributes = FILE_ATTRIBUTE_DIRECTORY.0;
            }
            EntryKind::File { size, .. } => {
                info.file_attributes = FILE_ATTRIBUTE_READONLY.0;
                info.file_size = *size as u64;
                info.allocation_size = ((*size as u64) + 511) & !511;
            }
        }
        
        info
    }
    
    fn resolve_path(
        registry: &InodeRegistry<'static, FK>,
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
    
    pub fn mount(self, mount_point: impl AsRef<std::path::Path>) -> FspResult<()>{
        use winfsp::{winfsp_init_or_die, host::{VolumeParams, FileSystemHost, CoarseGuard}};
        
        let init = winfsp_init_or_die();
        let mount_point = mount_point.as_ref().to_string_lossy().to_string();
        
        let fs = std::sync::Mutex::new(Some(self));
        
        winfsp::service::FileSystemServiceBuilder::new().with_start(move || {
            let fs = fs.lock().unwrap().take().ok_or_else(|| winfsp::FspError::from(
                windows::Win32::Foundation::STATUS_INVALID_PARAMETER
            ))?;
            
            let serial = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .subsec_nanos();
            
            let mut params = VolumeParams::new();
            params.sector_size(512);
            params.sectors_per_allocation_unit(1);
            params.max_component_length(255);
            params.volume_creation_time(0);
            params.file_info_timeout(1000);
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
            println!("{}", mount_point);
            host.mount(&mount_point)?;
            println!("bunda4");
            host.start()?;
            println!("bunda5");
            Ok(host)
        })
        .with_stop(|host| if let Some(h) = host { Ok(h.stop()) } else { Ok(()) })
        .build("m_s", init)?
        .start()
    }
}

impl<FileKind: FKind, Ctx: Context<'static, FileKind>> FileSystemContext
    for PakFilesystem<'static, FileKind, Ctx>
{
    type FileContext = PakFileContext;

    fn get_security_by_name(
        &self,
        file_name: &U16CStr,
        _security_descriptor: Option<&mut [std::ffi::c_void]>,
        _reparse_point_resolver: impl FnOnce(&U16CStr) -> Option<FileSecurity>,
    ) -> FspResult<FileSecurity> {
        println!("get_security_by_name: {}", file_name.to_string_lossy());
        let registry = self.registry.read();
        let ino = Self::resolve_path(&registry, file_name)
            .ok_or_else(|| FspError::from(windows::Win32::Foundation::STATUS_OBJECT_NAME_NOT_FOUND))?;
            
        println!("resolved {} -> {:?}", file_name.to_string_lossy(), ino);
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
        let ino = Self::resolve_path(&registry, file_name)
            .ok_or_else(|| FspError::from(windows::Win32::Foundation::STATUS_OBJECT_NAME_NOT_FOUND))?;

        let entry = registry.by_ino.get(&ino).unwrap();
        
        if granted_access & (FILE_WRITE_DATA.0 | FILE_APPEND_DATA.0) != 0 {
            return Err(FspError::from(STATUS_ACCESS_DENIED));
        }
        
        *file_info.as_mut() = Self::make_file_info(&entry.kind);
        if !entry.kind.is_dir() {
            if !self.cache.read().contains_key(&ino) {
                println!("Writing to cache!");
                let registry = self.registry.read();
                let entry = registry.by_ino.get(&ino)
                    .ok_or_else(|| FspError::from(windows::Win32::Foundation::STATUS_OBJECT_NAME_NOT_FOUND))?;
                let EntryKind::File { ref paths, ref parser_kind, .. } = entry.kind else {
                    return Err(FspError::from(windows::Win32::Foundation::STATUS_INVALID_PARAMETER));
                };
                let content = self.parse_file(parser_kind, paths)
                    .map_err(|_| FspError::from(windows::Win32::Foundation::STATUS_IO_DEVICE_ERROR))?;
                self.cache.write().insert(ino, content);
            }
    
            let cache = self.cache.read();
            let content = cache.get(&ino)
                .ok_or_else(|| FspError::from(windows::Win32::Foundation::STATUS_OBJECT_NAME_NOT_FOUND))?;
            // let finfo = file_info.as_mut();
            // finfo.file_size = content.len() as u64;
            // finfo.allocation_size = ((content.len() as u64) + 511) & !511;
        }
        
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

    fn read(
        &self,
        context: &Self::FileContext,
        buffer: &mut [u8],
        offset: u64,
    ) -> FspResult<u32> {
        if context.is_dir {
            return Err(FspError::from(windows::Win32::Foundation::STATUS_INVALID_PARAMETER));
        }
        
        if !self.cache.read().contains_key(&context.ino) {
            let registry = self.registry.read();
            let entry = registry.by_ino.get(&context.ino)
                .ok_or_else(|| FspError::from(windows::Win32::Foundation::STATUS_OBJECT_NAME_NOT_FOUND))?;
            let EntryKind::File { ref paths, ref parser_kind, .. } = entry.kind else {
                return Err(FspError::from(windows::Win32::Foundation::STATUS_INVALID_PARAMETER));
            };
            let content = self.parse_file(parser_kind, paths)
                .map_err(|_| FspError::from(windows::Win32::Foundation::STATUS_IO_DEVICE_ERROR))?;
            self.cache.write().insert(context.ino, content);
        }

        let cache = self.cache.read();
        let content = cache.get(&context.ino)
            .ok_or_else(|| FspError::from(windows::Win32::Foundation::STATUS_OBJECT_NAME_NOT_FOUND))?;

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
            let Some(entry) = registry.by_ino.get(&child_ino) else { continue };
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

    fn get_file_info(
        &self,
        context: &Self::FileContext,
        file_info: &mut FileInfo,
    ) -> FspResult<()> {
        let registry = self.registry.read();
        let entry = registry.by_ino.get(&context.ino)
            .ok_or_else(|| FspError::from(windows::Win32::Foundation::STATUS_OBJECT_NAME_NOT_FOUND))?;
        *file_info = Self::make_file_info(&entry.kind);
        Ok(())
    }

    fn get_volume_info(&self, volume_info: &mut winfsp::filesystem::VolumeInfo) -> FspResult<()> {
        let registry = self.registry.read();
        let total: u64 = registry.by_ino.values()
            .map(|e| e.kind.size())
            .sum();
        volume_info.total_size = total;
        volume_info.free_size = 0;
        // volume_info.set_volume_label("PakFS");
        
        Ok(())
    }
}
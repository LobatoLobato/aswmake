use path_clean::PathClean;

use crate::error;

pub trait Path: std::fmt::Debug {
   fn to_path_buf(&self) -> std::path::PathBuf;
   fn as_path(&self) -> &'_ std::path::Path;
   fn absolute(&self) -> anyhow::Result<std::path::PathBuf>;
   fn absolute_file(&self) -> anyhow::Result<std::path::PathBuf>;
   fn absolute_dir(&self) -> anyhow::Result<std::path::PathBuf>;
}
impl<T> Path for T where T: AsRef<std::path::Path> + std::fmt::Debug{
    fn to_path_buf(&self) -> std::path::PathBuf {
        self.as_ref().to_path_buf()
    }
    fn as_path(&self) -> &'_ std::path::Path {
        self.as_ref()
    }

    fn absolute(&self) -> anyhow::Result<std::path::PathBuf> {
        std::env::current_dir().map(|d| d.join(self).clean()).or(Err(error::InvalidFilePath(self)))
    }
    
    fn absolute_file(&self) -> anyhow::Result<std::path::PathBuf> {
        if let Ok(f) = self.absolute() && f.is_file() {Ok(f)} else {Err(error::InvalidFilePath(self))}
    }
    
    fn absolute_dir(&self) -> anyhow::Result<std::path::PathBuf> {
        if let Ok(f) = self.absolute() && f.is_dir() {Ok(f)} else {Err(error::InvalidFilePath(self))}
    }
    
}


pub auto trait NotOption {}
impl<T> !NotOption for Option<T> {}

pub trait OptionalPath {
    fn to_path_buf(&self) -> Option<std::path::PathBuf>;
    fn as_path(&self) -> Option<&'_ std::path::Path>;
    fn absolute(&self) -> Option<std::path::PathBuf>;
    fn absolute_file(&self) -> Option<std::path::PathBuf>;
    fn absolute_dir(&self) -> Option<std::path::PathBuf>;
}

impl<T: AsRef<std::path::Path> + NotOption> OptionalPath for T {
    fn to_path_buf(&self) -> Option<std::path::PathBuf> {
        Some(self.as_ref().to_path_buf())
    }
    fn as_path(&self) -> Option<&'_ std::path::Path> {
        Some(self.as_ref())
    }

    fn absolute(&self) -> Option<std::path::PathBuf> {
        std::env::current_dir().map(|d| d.join(self).clean()).ok()
    }
    fn absolute_file(&self) -> Option<std::path::PathBuf> {
        if let Some(f) = self.absolute() && f.is_file() {Some(f)} else {None}
    }
    fn absolute_dir(&self) -> Option<std::path::PathBuf> {
        if let Some(f) = self.absolute() && f.is_dir() {Some(f)} else {None}
    }
}
impl OptionalPath for Option<std::convert::Infallible> {
    fn to_path_buf(&self) -> Option<std::path::PathBuf> {
        None
    }
    fn as_path(&self) -> Option<&'_ std::path::Path> {
        None
    }

    fn absolute(&self) -> Option<std::path::PathBuf> {
        None
    }
    fn absolute_file(&self) -> Option<std::path::PathBuf> {
        None
    }
    fn absolute_dir(&self) -> Option<std::path::PathBuf> {
        None
    }
}
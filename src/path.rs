use path_clean::PathClean;

use crate::error;

pub trait Path: std::fmt::Debug {
   fn to_path_buf(&self) -> std::path::PathBuf;
   fn as_path(&self) -> &'_ std::path::Path;
   fn absolute(&self) -> anyhow::Result<std::path::PathBuf>;
   fn absolute_file(&self) -> anyhow::Result<std::path::PathBuf>;
   fn absolute_dir(&self) -> anyhow::Result<std::path::PathBuf>;
}
impl<T> Path for T where T: AsRef<std::path::Path> + std::fmt::Debug {
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

#[allow(non_upper_case_globals)]
pub const NoPath: Option<&str> = Option::None;

pub trait OptionalPath {
    fn to_path_buf(&self) -> Option<std::path::PathBuf>;
    fn as_path(&self) -> Option<&'_ std::path::Path>;
    fn absolute(&self) -> Option<std::path::PathBuf>;
    fn absolute_file(&self) -> Option<std::path::PathBuf>;
    fn absolute_dir(&self) -> Option<std::path::PathBuf>;
}

impl<T: Path> OptionalPath for Option<T> {
    fn to_path_buf(&self) -> Option<std::path::PathBuf> {
        self.as_ref().map(|p| p.to_path_buf())
    }

    fn as_path(&self) -> Option<&'_ std::path::Path> {
        self.as_ref().map(|p| p.as_path())
    }

    fn absolute(&self) -> Option<std::path::PathBuf> {
        let inner = self.as_ref()?;
        std::env::current_dir().map(|d| d.join(inner.as_path()).clean()).ok()
    }

    fn absolute_file(&self) -> Option<std::path::PathBuf> {
        self.absolute().filter(|f| f.is_file())
    }

    fn absolute_dir(&self) -> Option<std::path::PathBuf> {
        self.absolute().filter(|f| f.is_dir())
    }
}

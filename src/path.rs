pub trait Path: std::fmt::Debug {
   fn to_path_buf(&self) -> std::path::PathBuf;
   fn as_path(&self) -> &'_ std::path::Path;
}
impl<T> Path for T where T: AsRef<std::path::Path> + std::fmt::Debug{
    fn to_path_buf(&self) -> std::path::PathBuf {
        self.as_ref().to_path_buf()
    }
    fn as_path(&self) -> &'_ std::path::Path {
        self.as_ref()
    }
}


pub auto trait NotOption {}
impl<T> !NotOption for Option<T> {}

pub trait OptionalPath {
    fn to_path_buf(&self) -> Option<std::path::PathBuf>;
    fn as_path(&self) -> Option<&'_ std::path::Path>;
}

impl<T: AsRef<std::path::Path> + NotOption> OptionalPath for T {
    fn to_path_buf(&self) -> Option<std::path::PathBuf> {
        Some(self.as_ref().to_path_buf())
    }
    fn as_path(&self) -> Option<&'_ std::path::Path> {
        Some(self.as_ref())
    }
}
impl OptionalPath for Option<std::convert::Infallible> {
    fn to_path_buf(&self) -> Option<std::path::PathBuf> {
        None
    }
    fn as_path(&self) -> Option<&'_ std::path::Path> {
        None
    }
}
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Missing field \"{0}\" on {1}")]
    MissingField(String, String),
    
    #[error("Uexp type is unsupported for \"{0}\"")]
    UnsupportedUexpType(std::path::PathBuf)
}

#[allow(non_snake_case)]
pub fn MissingField(field: &str, on: &str) -> Error {
    Error::MissingField(field.into(), on.into())
}

#[allow(non_snake_case, dead_code)]
pub fn UnsupportedUexpType(file_path: &std::path::Path) -> Error {
    Error::UnsupportedUexpType(file_path.to_path_buf())
}
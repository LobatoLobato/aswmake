#[allow(non_snake_case)]
pub fn InvalidFilePath(path: impl crate::path::Path) -> anyhow::Error {
    if path.as_path().starts_with("/") || path.as_path().as_os_str().is_empty() {
        anyhow::Error::msg(format!("'{}' is not a valid file path", path.as_path().display()))    
    } else {
        anyhow::Error::msg(format!("'./{}' is not a valid file path", path.as_path().display()))    
    }   
}
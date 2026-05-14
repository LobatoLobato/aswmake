#[allow(non_snake_case)]
pub fn InvalidFilePath(path: impl crate::path::Path) -> anyhow::Error {
    anyhow::Error::msg(format!("{} is not a valid file path", path.as_path().display()))
}
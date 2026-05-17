
use std::{collections::HashMap, path::PathBuf};

use aswmake_lib::path::Path;
use confy::ConfyError;
use serde_derive::{Serialize, Deserialize};

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct GamePak {
    pub path: PathBuf,
    pub hash: String,
    pub ms_dir: PathBuf
}
impl GamePak {
    pub fn new(path: impl Path, ms_dir: impl Path) -> anyhow::Result<Self>{
        let hash = format!("{:?}", path.as_path().metadata()?.modified()?);
        Ok(Self {path: path.absolute()?, hash, ms_dir: ms_dir.absolute()?})
    }
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct ToolConfig {
    pub paks: HashMap<String, GamePak>
}
impl ToolConfig {
    const NAME: &str = "com.lobatolobato.aswmake";
    
    pub fn load() -> Result<ToolConfig, ConfyError> {
        confy::load::<ToolConfig>(ToolConfig::NAME, None)
    }    
    pub fn store(&self) -> Result<(), ConfyError> {
        confy::store(ToolConfig::NAME, None, self)
    }
    pub fn path(&self) -> PathBuf {
        confy::get_configuration_file_path(ToolConfig::NAME, None).unwrap()
    }
    pub fn dir(&self) -> PathBuf {
        self.path().parent().unwrap().to_path_buf()
    }
    pub fn ms_dir(&self) -> PathBuf {
        self.dir().join("ms")
    }
}


use std::{collections::HashMap, path::PathBuf};

use aswmake_lib::path::Path;
use confy::ConfyError;
use serde_derive::{Serialize, Deserialize};

use crate::m_s::INodeSizeIndex;

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct GamePak {
    pub path: PathBuf,
    pub inode_size_index: INodeSizeIndex
}
impl GamePak {
    pub fn new(path: impl Path, inode_size_index: INodeSizeIndex) -> anyhow::Result<Self>{
        let path = path.absolute_file()?;
        Ok(Self {path, inode_size_index})
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
}

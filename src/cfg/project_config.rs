use std::{collections::HashMap, path::PathBuf};

use aswmake_lib::path::Path;

use crate::error;

#[derive(Debug)]
pub struct ProjectConfig {
    pub project_name: String,
    pub src_dir: PathBuf,
    pub build_dir: PathBuf,
    pub ms_dir: PathBuf,
    pub target_game: aswmake_lib::TargetGame,
    pub game_pak_path: PathBuf,
    pub install_dir: Option<PathBuf>
}

impl ProjectConfig {
    pub fn new(
        project_name: String,
        target_game: aswmake_lib::TargetGame,
        game_pak_path: PathBuf,
        install_dir: Option<PathBuf>
    ) -> Self {
        let cwd = std::env::current_dir().unwrap();
        let project_dir = cwd.join(&project_name);
        Self {
            project_name,
            target_game: target_game.clone(),
            src_dir: project_dir.join("src"),
            build_dir: project_dir.join("build"),
            ms_dir: project_dir.join(target_game.to_string()),
            game_pak_path: game_pak_path,
            install_dir: install_dir
        }
    }

    pub fn load(cfg_path: impl Path) -> anyhow::Result<Self> {
        let mut cfg_path = cfg_path.absolute()?;
        if cfg_path.is_dir() { cfg_path = cfg_path.join("aswmake.toml"); }
        let dotenv_path = cfg_path.parent().unwrap().join(".env");
        let contents = std::fs::read_to_string(&cfg_path)?;

        let config: toml::Table = toml::from_str(&contents)?;
        let dotenv: HashMap<String, String> = dotenvy::from_path_iter(dotenv_path)?
            .collect::<Result<_, _>>()?;

        let to_string = |a: &toml::Value| a.as_str().unwrap().to_string();

        let project_name = config.get("name").map(to_string).ok_or(error::MissingField("name", "aswmake.toml"))?;
        let target_game = config.get("game").map(to_string).ok_or(error::MissingField("game", "aswmake.toml"))?;
        let src_dir = config.get("src_dir").map(to_string).ok_or(error::MissingField("src_dir", "aswmake.toml"))?;
        let ms_dir = config.get("ms_dir").map(to_string).ok_or(error::MissingField("ms_dir", "aswmake.toml"))?;
        let build_dir = config.get("build_dir").map(to_string).ok_or(error::MissingField("build_dir", "aswmake.toml"))?;

        Ok(Self {
            project_name,
            target_game: target_game.parse::<aswmake_lib::TargetGame>()?,
            src_dir: src_dir.to_path_buf(),
            ms_dir: ms_dir.to_path_buf(),
            build_dir: build_dir.to_path_buf(),
            game_pak_path: dotenv.get("ASWM_GAME_PAK_PATH").unwrap().to_path_buf(),
            install_dir: dotenv.get("ASWM_INSTALL_DIR").map(String::to_path_buf)
        })
    }
}

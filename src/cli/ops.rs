use std::path::PathBuf;

use color_print::cprintln;
use include_dir::{Dir, include_dir};
use lazy_regex::regex_replace;
use path_clean::PathClean as _;

use aswmake_lib::path::{NoPath, Path};

static TEMPLATE_DIR: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/templates");

pub fn scaffold(tool_cfg: &crate::cfg::ToolConfig, project_cfg: &crate::cfg::ProjectConfig) -> anyhow::Result<()> {
    let cwd = std::env::current_dir().unwrap();
    let project_dir = cwd.join(&project_cfg.project_name);
    let target_game = project_cfg.target_game.to_string();
    std::fs::create_dir_all(&project_dir)?;
    
    for entry in TEMPLATE_DIR.get_dir(&target_game).unwrap().find("**/*").unwrap() {
        if let Some(file) = entry.as_file() {
            let path = project_dir.join(file.path().strip_prefix(&target_game).unwrap());
            std::fs::create_dir_all(path.parent().unwrap())?;
            std::fs::write(path, file.contents())?;
        }
    }
    
    let aswmake_toml_path = project_dir.join("aswmake.toml");
    let aswmake_toml = std::fs::read_to_string(&aswmake_toml_path)?;
    std::fs::write(aswmake_toml_path, aswmake_toml
        .replace("{{PROJECT_NAME}}", &project_cfg.project_name)
        .replace("{{TARGET_GAME}}", &project_cfg.target_game.to_string())
    )?;
    
    std::fs::write(project_dir.join(".env"), format!("GAME_PAK_PATH=\"{}\"\nINSTALL_DIR=\"{}\"", 
        project_cfg.game_pak_path.display(),
        project_cfg.install_dir.as_ref().and_then(|d| d.to_str()).unwrap_or("\"/path/to/mods_folder\"")
    ))?;
    
    let game_ms_dir = tool_cfg.ms_dir().join(project_cfg.target_game.to_string());
    
    link_ms(game_ms_dir, &project_cfg.ms_dir)?;
    Ok(())
}

pub fn link_ms(ms_dir: impl Path, link_dir: impl Path) -> anyhow::Result<()> {
    let ms_dir = ms_dir.as_path();
    let link_dir = std::env::current_dir()?.join(link_dir.as_path()).clean();
    
    if let Ok(metadata) = link_dir.symlink_metadata() {
        let prompt_msg = format!("This operation will overwrite the existing {}. Continue?", 
            link_dir.file_name().unwrap().display()
        );
        let overwrite = inquire::Select::new(prompt_msg.as_str(), vec!["Yes", "No"])
            .without_filtering()
            .prompt()?;
        
        if overwrite == "No" { return Ok(()); }
        
        if metadata.is_dir() && !metadata.is_symlink() {
            std::fs::remove_dir_all(&link_dir)?;
        } else if cfg!(windows) && metadata.is_dir() && metadata.is_symlink() {
            std::fs::remove_dir(&link_dir)?;
        } else {
            std::fs::remove_file(&link_dir)?;
        }
    }
    
    #[cfg(unix)]
    std::os::unix::fs::symlink(ms_dir, link_dir)?;
    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(ms_dir, link_dir)?;
    Ok(())
}
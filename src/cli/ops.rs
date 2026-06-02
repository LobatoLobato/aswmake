use std::collections::HashMap;
use include_dir::{Dir, include_dir};
use itertools::Itertools;

use aswmake_lib::path::Path;

static TEMPLATE_DIR: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/templates");

pub fn scaffold(project_cfg: &crate::cfg::ProjectConfig) -> anyhow::Result<()> {
    let cwd = std::env::current_dir().unwrap();
    let project_dir = cwd.join(&project_cfg.project_name);
    let target_game = project_cfg.target_game.to_string();
    std::fs::create_dir_all(&project_dir)?;

    for entry in TEMPLATE_DIR.get_dir(&target_game).unwrap().find("**/*").unwrap() {
        if let Some(file) = entry.as_file() && !file.path().ends_with("aswmake.toml"){
            let path = project_dir.join(file.path().strip_prefix(&target_game).unwrap());
            std::fs::create_dir_all(path.parent().unwrap())?;
            std::fs::write(path, file.contents())?;
        }
    }

    write_aswmake_toml(project_cfg, &project_dir)?;
    write_dotenv(project_cfg, project_dir)?;

    Ok(())
}
pub fn scaffold_min(project_cfg: &crate::cfg::ProjectConfig) -> anyhow::Result<()> {
    let cwd = std::env::current_dir().unwrap();
    let project_dir = cwd.join(&project_cfg.project_name);

    write_aswmake_toml(project_cfg, &project_dir)?;
    write_dotenv(project_cfg, project_dir)?;

    Ok(())
}

fn write_aswmake_toml(project_cfg: &crate::cfg::ProjectConfig, project_dir: impl Path) -> anyhow::Result<()> {
    let target_game = project_cfg.target_game.to_string();
    let project_toml = project_dir.as_path().join("aswmake.toml");
    if project_toml.exists() {
        println!("{}/aswmake.toml already exists.", project_dir.as_path().file_name().unwrap().display());
        return Ok(());
    }

    let f = TEMPLATE_DIR.get_dir(&target_game).unwrap().find("**/aswmake.toml").unwrap().next().unwrap();
    let aswmake_toml_contents = f.as_file().unwrap().contents_utf8().unwrap();
    std::fs::write(project_toml, aswmake_toml_contents
        .replace("{{PROJECT_NAME}}", &project_cfg.project_name)
        .replace("{{TARGET_GAME}}", &target_game)
    )?;
    Ok(())
}
fn write_dotenv(project_cfg: &crate::cfg::ProjectConfig, project_dir: impl Path) -> anyhow::Result<()> {
    type DotEnv = HashMap<String, String>;
    let dotenv_path = project_dir.as_path().join(".env");
    let mut dotenv: DotEnv = HashMap::new();

    if dotenv_path.exists() {
        dotenv.extend(dotenvy::from_path_iter(&dotenv_path)?.collect::<Result<DotEnv, _>>()?);
    }

    if !dotenv.contains_key("ASWM_GAME_PAK_PATH") {
        let game_pak_path = project_cfg.game_pak_path.to_string_lossy().into_owned();
        dotenv.insert("ASWM_GAME_PAK_PATH".into(), game_pak_path);
    }
    if !dotenv.contains_key("ASWM_INSTALL_DIR") {
        let install_dir = project_cfg.install_dir.clone().unwrap_or("/path/to/mods_folder".to_path_buf());
        dotenv.insert("ASWM_INSTALL_DIR".into(), install_dir.to_string_lossy().into_owned());
    }

    let dotenv_content = dotenv.iter().map(|(k, v)| format!("{k}=\"{}\"", v.replace('"', ""))).join("\n");
    std::fs::write(dotenv_path, dotenv_content)?;

    Ok(())
}

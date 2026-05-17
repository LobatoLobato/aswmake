use std::path::PathBuf;

use aswmake_lib::path::Path;
use strum::VariantNames as _;

use crate::cli;

pub fn new(cfg: &mut crate::cfg::ToolConfig) -> anyhow::Result<()> {
    use inquire::{Text, Select};
    let project_name = Text::new("Project name:")
        .with_default("mod")
        .with_placeholder("mod")
        .prompt()?;
    
    let project_dir = std::env::current_dir().unwrap().join(&project_name);
    if project_dir.exists() {
        println!("⚠️ {} is not an empty directory.", project_dir.display());
        return Err(aswmake_lib::error::InvalidFilePath(project_name));
    }
    
    let target_game = Select::new("Select the game the mod is for:", aswmake_lib::TargetGame::VARIANTS.to_vec())
        .without_filtering()
        .prompt()?;
    
    let Some(game_pak) = cfg.paks.get(target_game) else {
        return Err(anyhow::format_err!("{target_game} doesn't have an m_s configured\n Run aswmake_lib ms add {target_game} '/path/to/game.pak' first"));
    };
    
    let game_paks_path = &game_pak.path.parent().unwrap().to_owned();    
    let install_dir = Text::new("Path to game mods directory:")
        .with_initial_value(&game_paks_path.to_string_lossy())
        .prompt_skippable()?;
    
    let pcfg = crate::cfg::ProjectConfig::new(
        project_name,
        target_game.parse::<aswmake_lib::TargetGame>()?,
        game_pak.path.clone(),
        install_dir.map(PathBuf::from)
    );
    
    cli::ops::scaffold(&cfg, &pcfg)?;
    
    Ok(())
}

pub fn ms(cfg: &mut crate::cfg::ToolConfig, command: &cli::MsCommands) -> anyhow::Result<()> {
    todo!();
}

pub fn build(cfg_root_path: impl Path) -> anyhow::Result<()> {
    let cfg = crate::cfg::ProjectConfig::load(cfg_root_path.as_path().join("aswmake.toml"))?;
    
    let compiled_dir = cfg.build_dir.join("compiled");
    let package_path = cfg.build_dir.join(&cfg.project_name).with_extension("pak");
    
    cli::ops::compile_and_package(cfg, compiled_dir, package_path)
}


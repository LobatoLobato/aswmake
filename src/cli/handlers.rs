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
    let use_scaffold = !project_dir.exists();
    
    if project_dir.is_dir() && project_dir.join("aswmake.toml").is_file() {
        println!("{} is already an aswmake project.", project_dir.display());
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
    
    if use_scaffold {
        cli::ops::scaffold(&cfg, &pcfg)?;
    } else {
        cli::ops::scaffold_min(&cfg, &pcfg)?;
    }
    
    Ok(())
}

pub fn ms(cfg: &mut crate::cfg::ToolConfig, command: &cli::MsCommands) -> anyhow::Result<()> {
    if let (is_valid, game) = command.validate_game() && !is_valid {
        println!("Invalid game \"{game}\". The possible keys are:");
        for game in aswmake_lib::TargetGame::VARIANTS { println!(">  {game}"); }
        return Ok(());
    }
    
    match command {
        cli::MsCommands::Add { game, pak_path } => {
            let ms_path = cfg.ms_dir().join(&game);
            let target_game = game.parse::<aswmake_lib::TargetGame>()?;
            cli::ops::m_s(&pak_path, PathBuf::from(&ms_path), target_game)?;
            cfg.paks.insert(game.to_owned(), crate::cfg::GamePak::new(pak_path, ms_path)?);
            
            cfg.store()?;
        },
        cli::MsCommands::Remove { game } => {
            if let Some(pak) = cfg.paks.remove(game) {
                std::fs::remove_dir_all(pak.ms_dir)?;
                cfg.store()?;    
            }
        }
        cli::MsCommands::Link { game } => {
            cli::ops::link_ms(cfg.ms_dir().join(game), game)?;
        }
        cli::MsCommands::Update { game } => {
            if let Some(pak) = cfg.paks.get(game) {
                let target_game = game.parse::<aswmake_lib::TargetGame>()?;
                cli::ops::m_s(&pak.path, cfg.ms_dir().join(game), target_game)?;
            } else {
                return Err(anyhow::format_err!("No registered pak path for {game}"));
            }
        }
    };
    Ok(())
}

pub fn build(cfg_root_path: impl Path) -> anyhow::Result<()> {
    let cfg = crate::cfg::ProjectConfig::load(cfg_root_path.as_path().join("aswmake.toml"))?;
    
    let compiled_dir = cfg.build_dir.join("compiled");
    let package_path = cfg.build_dir.join(&cfg.project_name).with_extension("pak");
    
    cli::ops::compile_and_package(cfg, compiled_dir, package_path)
}


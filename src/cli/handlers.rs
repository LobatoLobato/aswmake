use std::path::PathBuf;

use aswmake_lib::path::{OptionalPath, Path};
use color_print::cprintln;
use path_clean::PathClean;
use strum::VariantNames as _;

use crate::{cfg::ProjectConfig, cli, compiler::Compiler, context, m_s};

pub fn new(cfg: &mut crate::cfg::ToolConfig, path: &Option<String>) -> anyhow::Result<()> {
    use inquire::{Select, Text};
    
    let cwd = std::env::current_dir()?;
    let project_dir = path.as_ref().map(|path| cwd.join(path).clean());
    
    let project_name = Text::new("Project name:")
        .with_default("mod")
        .with_placeholder("mod")
        .prompt()?;

    let project_dir = project_dir.unwrap_or_else(|| cwd.join(&project_name).clean());
    let use_scaffold = !project_dir.exists();

    if project_dir.is_dir() && project_dir.join("aswmake.toml").is_file() {
        println!("{} is already an aswmake project.", project_dir.display());
        return Err(aswmake_lib::error::InvalidFilePath(project_name));
    }

    cprintln!("<green>></green> Project dir: <cyan>{}</cyan>", project_dir.display());
    
    
    let target_game = Select::new(
        "Select the game the mod is for:",
        aswmake_lib::TargetGame::VARIANTS.to_vec(),
    )
    .without_filtering()
    .prompt()?;

    let Some(game_pak) = cfg.paks.get(target_game) else {
        return Err(anyhow::format_err!(
            "{target_game} doesn't have an m_s configured\n Run aswmake_lib ms add {target_game} '/path/to/game.pak' first"
        ));
    };

    let game_paks_path = &game_pak.path.parent().unwrap().to_owned();
    let install_dir = Text::new("Path to game mods directory:")
        .with_initial_value(&game_paks_path.to_string_lossy())
        .prompt_skippable()?;

    let pcfg = crate::cfg::ProjectConfig::new(
        project_name,
        target_game.parse::<aswmake_lib::TargetGame>()?,
        game_pak.path.clone(),
        install_dir.map(PathBuf::from),
    );

    if use_scaffold {
        cli::ops::scaffold(&pcfg, project_dir)?;
    } else {
        cli::ops::scaffold_min(&pcfg, project_dir)?;
    }

    Ok(())
}

pub fn ms(cfg: &mut crate::cfg::ToolConfig, command: &cli::MsCommands) -> anyhow::Result<()> {
    if let (is_valid, game) = command.validate_game()
        && !is_valid
    {
        println!("Invalid game \"{game}\". The possible keys are:");
        for game in aswmake_lib::TargetGame::VARIANTS {
            println!(">  {game}");
        }
        return Ok(());
    }

    match command {
        cli::MsCommands::Add { game, pak_path } => {
            let inode_size_index = m_s::PakFilesystem::new(pak_path.to_path_buf(), context::new())?
                .generate_size_index(None)?;

            cfg.paks.insert(
                game.to_owned(),
                crate::cfg::GamePak::new(pak_path, inode_size_index)?,
            );

            cfg.store()?;
        }
        cli::MsCommands::Remove { game } => {
            if let Some(_) = cfg.paks.remove(game) {
                cfg.store()?;
            }
        }
        cli::MsCommands::Mount { game, mount_point } => {
            let cwd = std::env::current_dir()?;
            let mount_point = mount_point.absolute().unwrap_or_else(|| ProjectConfig::load(cwd.join("aswmake.toml"))
                .map(|c| cwd.join(c.ms_dir))
                .unwrap_or(cwd.join(game))
            );

            if let Some(pak) = cfg.paks.remove(game) {
                println!("Loading pak file...");
                let mut fs = m_s::PakFilesystem::new(pak.path, context::new())?;

                println!("Initializing file system...");
                fs.init(None, Some(pak.inode_size_index))?;

                println!("File system mounted at {}", mount_point.display());
                #[allow(unused_mut)]
                let mut session = fs.mount(&mount_point)?;
                let (tx, rx) = std::sync::mpsc::channel();

                ctrlc::set_handler(move || {
                    let _ = tx.send(());
                })?;

                if rx.recv().is_ok() {
                    println!("Unmounting file system...");
                    #[cfg(target_os = "linux")] {
                        session.umount_and_join()?;
                    }
                    #[cfg(target_os = "windows")] {
                        session.stop();
                        session.join()?;
                    }
                    println!("Removing {}...", mount_point.display());
                    std::fs::remove_dir(mount_point)?;
                }

                println!("Done :)")
            }
        }
        cli::MsCommands::Update { game } => {
            if let Some(pak) = cfg.paks.get_mut(game) {
                pak.inode_size_index = m_s::PakFilesystem::new(pak.path.clone(), context::new())?
                    .generate_size_index(None)?;

                cfg.store()?;
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

    let compiler = Compiler::new(cfg.game_pak_path, context::new())?;

    compiler.compile(cfg.src_dir, &compiled_dir)?;

    compiler.package(compiled_dir, package_path, cfg.install_dir)?;
    
    Ok(())
}

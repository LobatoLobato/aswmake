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

pub fn m_s(game_pak_path: impl Path, ms_dir: impl Path, target_game: aswmake_lib::TargetGame) -> anyhow::Result<()> {    
    let game_pak_path = game_pak_path.absolute_file()?;
    let ms_dir = ms_dir.absolute()?;
    
    std::fs::create_dir_all(&ms_dir)?;
    
    let loc_file_path = aswmake_lib::tools::repak::extract(
        &game_pak_path, 
        &ms_dir,
        target_game.aes_key(),
        Some(&["**/Localization/INT/REDGame.uexp"]),
        |rel_file_path, _| {println!("Extracting {rel_file_path}");},
        |_| {}
    )?[0].clone();
    
    println!("Processing loc file...");
    let loc = aswmake_lib::parsers::loc::parse(loc_file_path, NoPath)?; 
    
    println!("Extracting game files...");
    aswmake_lib::tools::repak::extract(
        &game_pak_path, 
        &ms_dir,
        target_game.aes_key(),
        Some(&[
            "**/Chara/**/Data/**/BBS_*",
            "**/Chara/**/Data/**/COL_*"
        ]),
        |rel_file_path, _| {println!("Extracting {rel_file_path}...");},
        |p| {
            let file_name = p.file_name().unwrap().to_string_lossy();
            let extension = p.extension().unwrap();
            let parent = p.parent().unwrap();
            let mut parsed_file_ext: Option<&str> = None;
            let mut bbs = None;
            
            if extension != "uexp" { 
                if extension == "uasset" && !parent.ends_with("Data"){
                    let _ = std::fs::remove_file(p);    
                }
                return; 
            }
            
            if file_name.starts_with("BBS_") {
                println!("Processing {}", file_name);
                
                match aswmake_lib::parsers::bbs::parse(p, NoPath, target_game.clone(), Some(&loc)) {
                    Ok(b) => bbs = Some(b),
                    Err(e) => return eprintln!("Error parsing BBS {}: {:?}", file_name, e)
                }
                parsed_file_ext = Some("bbs");
            } else if file_name.starts_with("COL_") {
                println!("Processing {}", file_name);
                
                let normalized_name = regex_replace!(r"(_\d+)?\.uexp", &file_name, ".pac").to_string();
                if let Err(e) = aswmake_lib::tools::bbspack::extract(p, parent.join(normalized_name)) {
                    eprintln!("Error extracting COL {}: {:?}", file_name, e);
                    return;
                }
                parsed_file_ext = Some("pac");
            }
            
            if !parent.ends_with("Data") {
                let _ = std::fs::remove_file(p.with_extension("uexp"));
            } else if let Some(parsed_file_ext) = parsed_file_ext {
                if let Some(bbs) = bbs && !file_name.ends_with("EF.uexp") {
                    let _ = std::fs::write(parent.join("movelist.json"), bbs.render());
                }
                let dest_dir = parent.join("current");
                let parsed_file = p.with_extension(parsed_file_ext);
                let parsed_file_dest = dest_dir.join(file_name.as_ref()).with_extension(parsed_file_ext);
                let _ = std::fs::create_dir_all(&dest_dir);
                let _ = std::fs::rename(&parsed_file, parsed_file_dest);
            }
        }
    )?;
    
    println!("Done.");
    
    Ok(())
}

pub fn compile_and_package(cfg: crate::cfg::ProjectConfig, compiled_dir: PathBuf, package_path: PathBuf) -> anyhow::Result<()> {
    aswmake_lib::build::compile_against_ms(&cfg.src_dir, &cfg.ms_dir, &compiled_dir, cfg.target_game,
        Some(|file_name, result| {
            if let Some(result) = result {
                result.lines().for_each(|l| { cprintln!("<yellow>></yellow><green> {l} </green>"); });
                cprintln!("<blue>-----------------------------------------</blue>");    
            } else {
                cprintln!("<yellow>Compiling {file_name}</yellow>...");
            }
        })
    )?;
    
    aswmake_lib::build::package(package_path, compiled_dir, cfg.install_dir.as_ref())?;
    
    Ok(())
}
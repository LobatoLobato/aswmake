// use aswmake::loc::Loc;
// use aswmake::tools;

use std::{collections::BTreeSet, path::PathBuf};

use aswmake::{parsers::Parser, tools::u4pak};
use color_print::cprintln;
use walkdir::WalkDir;

fn get_dir_structure(root: &std::path::Path) -> BTreeSet<PathBuf> {
    WalkDir::new(root).into_iter().filter_map(|e| e.ok()).map(|e| {
        e.path().strip_prefix(root).unwrap().to_path_buf()
    }).collect()    
}

//=============================\\
// This is not working code !!!
//=============================\\
fn main() -> anyhow::Result<()> {
    let game_pak_path = PathBuf::from("path/to/game.pak");
    let bms_dir = PathBuf::from("");
    
    /* m_s */ {
        aswmake::tools::quickbms::extract(
            &game_pak_path, 
            &bms_dir, 
            Some(&[
                aswmake::parsers::loc::Loc::bms_filter(),
                aswmake::parsers::bbs::BBS::bms_filter(),
                "{}/Chara/{}Data/{}COL_{}"
            ])
        )?;
        
        let loc = aswmake::parsers::loc::parse(
            bms_dir.join("/RED/Content/Localization/INT"), 
            None
        )?;
        
        for p in get_dir_structure(&bms_dir).iter().filter(|e| e.is_file()) {
            let file_name = p.file_name().unwrap().to_string_lossy();
            let mut parsed_file_ext: Option<&str> = None;
            if file_name.starts_with("BBS_") {
                aswmake::parsers::bbs::parse(p, None, Some(&loc))?;
                parsed_file_ext = Some(".bbs");
            } else if file_name.starts_with("COL_") {
                aswmake::tools::bbspack::extract(p, p.with_extension("pac"))?;
                parsed_file_ext = Some(".pac");
            }
            
            let parent = p.parent().unwrap();
            if !parent.ends_with("Data") {
                let _ = std::fs::remove_file(p.with_extension("uexp"));
                let _ = std::fs::remove_file(p.with_extension("uasset"));
            } else if let Some(parsed_file_ext) = parsed_file_ext {
                let dest_dir = parent.join("current");
                let parsed_file = p.with_extension(parsed_file_ext);
                let parsed_file_dest = dest_dir.join(file_name.as_ref()).with_extension(parsed_file_ext);
                let _ = std::fs::create_dir_all(&dest_dir);
                let _ = std::fs::rename(parsed_file, parsed_file_dest);
            }
        }   
    }
    
    /* build */ {
        let src_dir = "src";
        let out_dir = PathBuf::from("build");
        let out_pak = out_dir.join("proj_name.pak");
        
        aswmake::build::compile_against_bms(src_dir, bms_dir, &out_dir.join("compiled"),
            Some(|file_name, result| {
                if let Some(result) = result {
                    result.lines().for_each(|l| { cprintln!("<yellow>></yellow><green> {l} </green>"); });
                    cprintln!("<blue>-----------------------------------------</blue>");    
                } else {
                    cprintln!("<yellow>Compiling {file_name}</yellow>...");
                }
            })
        )?;
        
        aswmake::build::package(&out_pak, &out_dir.join("compiled"), game_pak_path)?;
    }
    
    Ok(())
}

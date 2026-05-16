use std::sync::LazyLock;

use crate::path::Path;
use crate::tools::{bbscript, bbspack, u4pak};

pub enum FileKind {
    BBSCRIPT,
    PAC
}

pub fn compile(
    input_path: impl Path, 
    uexp_path: impl Path, 
    uasset_path: impl Path, 
    out_dir: impl Path,
    target_game: crate::TargetGame,
    file_kind: FileKind,
    hook_fn: Option<fn(&str, Option<&str>)>
) -> anyhow::Result<(String, String)> {
    let input_path = input_path.absolute_file()?;
    let uexp_path = uexp_path.absolute_file()?; 
    let uasset_path = uasset_path.absolute_file()?;
    
    let input_dir = input_path.parent().unwrap();
    let out_dir = out_dir.absolute()?;
    std::fs::create_dir_all(&out_dir.as_path())?;
    
    if input_dir == out_dir  { return Err(anyhow::Error::msg("input_dir is the same as out_dir")); }
    
    
    let out_uexp_path = out_dir.join(uexp_path.file_name().unwrap());
    let out_uasset_path = out_dir.join(uasset_path.file_name().unwrap());
    let _ = std::fs::copy(&uexp_path, &out_uexp_path);
    let _ = std::fs::copy(&uasset_path, &out_uasset_path);
    
    let file_name = input_path.file_name().unwrap().to_string_lossy().into_owned();
    let output_path = out_dir.join(&file_name);
    
    hook_fn.iter().for_each(|f| f(&file_name, None));
    let r = match file_kind {
        FileKind::BBSCRIPT => {
            let rebuild_path = output_path.with_extension("bbscript");
            bbscript::rebuild(&input_path, &rebuild_path, target_game)?;
            let r = bbspack::inject(&rebuild_path, out_uexp_path, out_uasset_path)?;
            let _ = std::fs::remove_file(rebuild_path);
            Ok(r)
        },
        FileKind::PAC => bbspack::inject(input_path, out_uexp_path, out_uasset_path)
    }?;
    hook_fn.iter().for_each(|f| f(&file_name, Some(&r)));
    
    Ok((file_name, r))
}

pub fn compile_against_bms(
    input_dir: impl Path, 
    bms_dir: impl Path, 
    out_dir: impl Path,
    target_game: crate::TargetGame,
    hook_fn: Option<fn(&str, Option<&str>)>
) -> anyhow::Result<Vec<(String, String)>> {
    let input_dir = input_dir.absolute_dir()?;
    let bms_dir = bms_dir.absolute_dir()?;
    
    
    let out_dir = out_dir.absolute()?;
    std::fs::create_dir_all(&out_dir.as_path())?;
    
    if input_dir == out_dir && bms_dir == out_dir { 
        return Err(anyhow::Error::msg("input_dir/bms_dir is the same as out_dir")); 
    }
    
    let mut results = vec![];
    for entry in walkdir::WalkDir::new(&input_dir).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            let Some(file_name) = entry.file_name().to_str() else { continue; };
            let file_kind = if file_name.starts_with("BBS_") {
                FileKind::BBSCRIPT
            } else if file_name.starts_with("COL_") {
                FileKind::PAC
            } else {
                continue;
            };
            
            let Ok(file_rel) = entry.path().strip_prefix(&input_dir) else { continue; };
            let uexp_path = bms_dir.join(file_rel).with_extension("uexp");
            let uasset_path = bms_dir.join(file_rel).with_extension("uasset");
            let out_dir = out_dir.join(file_rel).parent().unwrap().to_path_buf();
            if !uexp_path.exists() && !uasset_path.exists() { continue; }
            
            let (file_name, r) = compile(
                entry.path(), uexp_path, uasset_path, out_dir, 
                target_game.clone(), file_kind, 
                hook_fn
            )?;
            
            results.push((file_name, r));
        }
    }
    
    Ok(results)
}

static SIG_FILE: LazyLock<tempfile::NamedTempFile> = LazyLock::new(|| {
    let sig = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/pak.sig"));
    let sig_file = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(sig_file.path(), sig).unwrap(); 
    
    sig_file
});

pub fn package(
    dest_pak_path: impl Path, 
    compiled_root_path: impl Path,
    install_dir: Option<impl Path>
) -> anyhow::Result<()> {
    let compiled_root_path = compiled_root_path.absolute_dir()?;
    let dest_pak_path = dest_pak_path.absolute()?;
    let sig_file_path = dest_pak_path.with_extension("sig");
    let build_dir = dest_pak_path.parent().unwrap();
    
    std::fs::create_dir_all(&build_dir)?;
    
    u4pak::pack(&dest_pak_path, compiled_root_path)?;
    std::fs::copy(SIG_FILE.path(), &sig_file_path)?;
    
    let package_name = dest_pak_path.file_stem().unwrap().to_string_lossy().into_owned();
    if let Some(install_dir) = install_dir.map(|d| d.as_path().join(&package_name)) {
        std::fs::create_dir_all(&install_dir)?;
        std::fs::copy(&dest_pak_path, &install_dir.join(&package_name).with_extension("pak"))?;
        std::fs::copy(&sig_file_path, &install_dir.join(&package_name).with_extension("sig"))?;
    }
    Ok(())
}


#[cfg(test)]
use suitest::{suite, suite_cfg};

#[cfg(test)]
#[suite(build_rs)]
#[suite_cfg(sequential = true, verbose = false)]
mod tests {
    use crate::{TargetGame, path::NoPath, util::sha1_hash};

    use super::*;
    use std::{path::PathBuf, sync::Arc};
    use suitest::{before_all};
    use tempfile;
    use walkdir::WalkDir;
    use std::collections::BTreeSet;
    
    fn get_dir_structure(root: &std::path::Path) -> BTreeSet<PathBuf> {
        WalkDir::new(root).into_iter().filter_map(|e| e.ok()).map(|e| {
            e.path().strip_prefix(root).unwrap().to_path_buf()
        }).collect()    
    }
    
    #[derive(Debug)]
    struct Context {
        _fixtures_dir: tempfile::TempDir,
        fixtures_dir_path: PathBuf,
        bms_dir: PathBuf,
        out_dir: PathBuf
    }
    
    #[before_all]
    fn setup() -> (Arc<Context>, ()){
        let (tmp_fixtures_dir, tmp_fixtures_dir_path) = crate::tests::make_temp_fixtures(Some("build"));
        let out_dir = tmp_fixtures_dir_path.join("build");
        
        (Arc::new(Context { 
            _fixtures_dir: tmp_fixtures_dir,
            bms_dir: tmp_fixtures_dir_path.join("bms"),
            out_dir: out_dir,
            fixtures_dir_path: tmp_fixtures_dir_path
        }), ())
    }
    
    #[test]
    fn can_compile_a_bbscript_file(ctx: Arc<Context>) {
        let uexp_path = ctx.fixtures_dir_path.join("single/BBS_FAU.uexp");
        let uasset_path = ctx.fixtures_dir_path.join("single/BBS_FAU.uasset");
        
        let input_file = ctx.fixtures_dir_path.join("single/equal/BBS_FAU.bbs");
        let out_dir = ctx.out_dir.join("single/equal");
        
        compile(input_file, &uexp_path, &uasset_path, &out_dir, TargetGame::GGST, FileKind::BBSCRIPT, None).unwrap();
        
        assert_eq!(sha1_hash(&uexp_path).ok(), sha1_hash(out_dir.join("BBS_FAU.uexp")).ok());
        assert_eq!(sha1_hash(&uasset_path).ok(), sha1_hash(out_dir.join("BBS_FAU.uasset")).ok());
        
        let input_file = ctx.fixtures_dir_path.join("single/different/BBS_FAU.bbs");
        let out_dir = ctx.out_dir.join("single/different");
        
        compile(input_file, &uexp_path, &uasset_path, &out_dir, TargetGame::GGST, FileKind::BBSCRIPT, None).unwrap();
        
        assert_ne!(sha1_hash(&uexp_path).ok(), sha1_hash(out_dir.join("BBS_FAU.uexp")).ok());
        assert_ne!(sha1_hash(&uasset_path).ok(), sha1_hash(out_dir.join("BBS_FAU.uasset")).ok());
    }
    
    #[test]
    fn can_compile_a_pac_file(ctx: Arc<Context>) {
        let uexp_path = ctx.fixtures_dir_path.join("single/COL_FAU.uexp");
        let uasset_path = ctx.fixtures_dir_path.join("single/COL_FAU.uasset");
        
        let input_file = ctx.fixtures_dir_path.join("single/equal/COL_FAU.pac");
        let out_dir = ctx.out_dir.join("single/equal");
        
        compile(input_file, &uexp_path, &uasset_path, &out_dir, TargetGame::GGST, FileKind::PAC, None).unwrap();
        
        assert_eq!(sha1_hash(&uexp_path).ok(), sha1_hash(out_dir.join("COL_FAU.uexp")).ok());
        assert_eq!(sha1_hash(&uasset_path).ok(), sha1_hash(out_dir.join("COL_FAU.uasset")).ok());
        
        let input_file = ctx.fixtures_dir_path.join("single/different/COL_FAU.pac");
        let out_dir = ctx.out_dir.join("single/different");
        
        compile(input_file, &uexp_path, &uasset_path, &out_dir, TargetGame::GGST, FileKind::PAC, None).unwrap();
        
        assert_ne!(sha1_hash(&uexp_path).ok(), sha1_hash(out_dir.join("COL_FAU.uexp")).ok());
        assert_ne!(sha1_hash(&uasset_path).ok(), sha1_hash(out_dir.join("COL_FAU.uasset")).ok());
    }
    
    #[test]
    fn can_compile_against_bms(ctx: Arc<Context>) {
        
        let input_dir = ctx.fixtures_dir_path.join("src/equal");
        let out_dir = ctx.out_dir.join("against_bms/equal");
        
        compile_against_bms(input_dir, &ctx.bms_dir, &out_dir, TargetGame::GGST, None).unwrap();
        assert!(!dir_diff::is_different(out_dir, &ctx.bms_dir).unwrap());
        
        let input_dir = ctx.fixtures_dir_path.join("src/different");
        let out_dir = ctx.out_dir.join("against_bms/different");
        compile_against_bms(input_dir, &ctx.bms_dir, &out_dir, TargetGame::GGST, None).unwrap();
        
        let out_dir_structure = get_dir_structure(&out_dir);
        let bms_dir_structure = get_dir_structure(&ctx.bms_dir);
        
        // Structure is the same but all file's contents are different
        assert_eq!(out_dir_structure, bms_dir_structure);
        for rel_path in out_dir_structure.iter().filter(|p| p.is_file()) {
            let out_path = out_dir.join(&rel_path);
            let bms_path = ctx.bms_dir.join(&rel_path);
            
            assert_ne!(sha1_hash(out_path).ok(), sha1_hash(bms_path).ok());   
        }
    }
    
    #[test]
    fn can_package_and_install_built_project(ctx: Arc<Context>) {
        let out_dir = ctx.out_dir.join("packaged");
        let out_pak = out_dir.join("foo.pak");
        let out_sig = out_dir.join("foo.sig");
        
        package(&out_pak, &ctx.bms_dir, NoPath).unwrap();
        
        assert!(std::fs::exists(&out_pak).is_ok());
        assert!(std::fs::exists(&out_sig).is_ok());
        
        let install_dir = ctx.fixtures_dir_path.join("install");
        let installed_pak = install_dir.join("foo.pak");
        let installed_sig = install_dir.join("foo.sig");
        
        package(&out_pak, &ctx.bms_dir, Some(install_dir)).unwrap();
        assert!(std::fs::exists(&out_pak).is_ok());
        assert!(std::fs::exists(&out_sig).is_ok());
        assert!(std::fs::exists(&installed_pak).is_ok());
        assert!(std::fs::exists(&installed_sig).is_ok());
        
    }
}
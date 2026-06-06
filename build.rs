use std::path::PathBuf;

use target_build_utils::TargetInfo;

fn main() {
    let target = TargetInfo::new().unwrap();
    let target_triplet = std::env::var("TARGET").unwrap();
    let profile = std::env::var("PROFILE").unwrap();
    
    let root_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let assets_dir = root_dir.join("assets");
    let tgt_dir = root_dir.join("target");
    
    let distrib_dir = tgt_dir.join("distrib").join(format!("aswmake-{target_triplet}"));
    let Some(bin_dir) = [
        tgt_dir.join(&target_triplet).join(&profile),
        tgt_dir.join(&profile),
        tgt_dir.join(&target_triplet).join("dist"),
        tgt_dir.join("dist"),
    ].into_iter().find(|path| out_dir.starts_with(path)) else {
        panic!("{}\n {}\n {}", profile, target_triplet, out_dir.display());
    };
    
    if target.target_os() == "windows" {
        let winfsp_dll = match target.target_arch() {
            "x86_64" => "winfsp-x64.dll",
            "x86" => "winfsp-x86.dll",
            "aarch64" => "winfsp-a64.dll",
            _ => panic!("unsupported architecture"),
        };
        let winfsp_dll_path = assets_dir.join("winfsp").join(winfsp_dll);
        if let Err(e) = std::fs::copy(&winfsp_dll_path, bin_dir.join(winfsp_dll)) {
            panic!("Failed to copy WinFSP DLL to binary directory: {}", e);
        } else {
            println!("cargo:rustc-link-lib=dylib=delayimp");
            println!("cargo:rustc-link-arg=/DELAYLOAD:{winfsp_dll}");
        }
        
        let _ = std::fs::copy(&winfsp_dll_path, distrib_dir.join(winfsp_dll));
    } else if target.target_os() == "linux" {
    }
}

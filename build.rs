
 
// #[cfg(target_os = "windows")] 
fn main() {
    // let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    #[cfg(target_os = "windows")] {
        winfsp::build::winfsp_link_delayload();
        
        let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
        let tgt_dir = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("target");
        let dll_src = tgt_dir.join("/winfsp/winfsp-x64.dll");
        let dll_dest = out_dir.join("winfsp-x64.dll");
        if let Err(e) = std::fs::copy(&dll_src, &dll_dest) {
            println!("cargo:warning=Failed to copy WinFSP DLL: {}", e);
        } else {
            println!("cargo:warning=Successfully copied WinFSP DLL to {:?}", dll_dest);
        }
    }
    // let profile_dir = out_dir
    //     .ancestors()
    //     .nth(3)
    //     .expect("Failed to find profile directory");

    // let build_dir = profile_dir.join("build");
    // if let Ok(entries) = fs::read_dir(&build_dir) {
    //     for entry in entries.flatten() {
    //         let path = entry.path();
    //         if path.is_dir() {
    //             if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
    //                 if name.starts_with("winfsp-sys-") {
    //                     let dll_src = path.join("out").join("bin").join("winfsp-x64.dll");
                        
    //                     if dll_src.exists() {
    //                         let dll_dest = profile_dir.join("winfsp-x64.dll");
                            
    //                         // Copy the DLL to the same folder as your compiled .exe
    //                         if let Err(e) = fs::copy(&dll_src, &dll_dest) {
    //                             println!("cargo:warning=Failed to copy WinFSP DLL: {}", e);
    //                         } else {
    //                             println!("cargo:warning=Successfully copied WinFSP DLL to {:?}", dll_dest);
    //                         }
    //                         break;
    //                     }
    //                 }
    //             }
    //         }
    //     }
    // }

    // // Ensure the build script triggers again if the build directory changes
    // println!("cargo:rerun-if-changed={}", build_dir.display());
}

// #[cfg(target_os = "linux")] fn main() {
// }
fn main() {
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
}

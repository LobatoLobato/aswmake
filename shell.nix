let 
    rust-overlay = builtins.fetchTarball "https://github.com/oxalica/rust-overlay/archive/master.tar.gz"; 
    pkgs = import <nixpkgs> { 
        overlays = [(import rust-overlay)]; 
    }; 
    
    toolchain = pkgs.rust-bin.fromRustupToolchainFile ./toolchain.toml;
    
in pkgs.mkShell { 
    nativeBuildInputs = with pkgs; [ 
        pkg-config 
        
        cargo-nextest
        cargo-make
        cargo-expand
        
        cargo-xwin
        cargo-zigbuild
        cargo-dist
        
        zig
        llvmPackages_latest.llvm 
        llvmPackages_latest.clang-unwrapped 
        llvmPackages_latest.libclang 
    ]; 
    buildInputs = with pkgs; [ 
        fuse 
    ]; 
    packages = [ 
        toolchain 
    ];
    
    RUST_SRC_PATH = "${toolchain}/lib/rustlib/src/rust/library"; 
    XWIN_CACHE_DIR = ".xwin-cache"; 
    LIBCLANG_PATH = "${pkgs.llvmPackages_latest.libclang.lib}/lib"; 
    
    CC_x86_64_pc_windows_msvc = "${pkgs.llvmPackages_latest.clang-unwrapped}/bin/clang-cl"; 
    CXX_x86_64_pc_windows_msvc = "${pkgs.llvmPackages_latest.clang-unwrapped}/bin/clang-cl"; 
    AR_x86_64_pc_windows_msvc = "${pkgs.llvmPackages_latest.llvm}/bin/llvm-lib";
    
    shellHook = ''
        cargo-dist() {
            local dist_bin
            dist_bin=$(ls -d /nix/store/*cargo-dist*/bin/dist 2>/dev/null | head -n 1)
        
            if [ -n "$dist_bin" ]; then
                "$dist_bin" "$@"
            else
                echo "Error: cargo-dist not found in /nix/store/" >&2
                return 1
            fi
        }
        build-win() {
            env NIX_CFLAGS_COMPILE="" NIX_LDFLAGS="" RUSTFLAGS="-C target-feature=+crt-static" cargo xwin build --release --target x86_64-pc-windows-msvc "$@"
        }
    '';
}

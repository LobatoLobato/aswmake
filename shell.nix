let 
    rust-overlay = builtins.fetchTarball "https://github.com/oxalica/rust-overlay/archive/master.tar.gz"; 
    pkgs = import <nixpkgs> { 
        overlays = [(import rust-overlay)]; 
    }; 
    toolchain = pkgs.rust-bin.fromRustupToolchainFile ./toolchain.toml;
    build-musl = pkgs.writeShellScriptBin "build-musl" ''
        export TMP_LINKER_DIR="$TMPDIR/musl-ldl-stub"
        mkdir -p "$TMP_LINKER_DIR"
        ${pkgs.stdenv.cc.bintools}/bin/ar rcs "$TMP_LINKER_DIR/libdl.a"

        env CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER="${pkgs.musl.dev}/bin/musl-gcc" \
            NIX_CFLAGS_COMPILE="-U_FORTIFY_SOURCE -D_FORTIFY_SOURCE=0" \
            RUSTFLAGS="-L native=$TMP_LINKER_DIR" \
            cargo build --release --target x86_64-unknown-linux-musl "$@"
    '';
in pkgs.mkShell { 
    nativeBuildInputs = with pkgs; [ 
        pkg-config 
        cargo-xwin 
        llvmPackages_latest.llvm 
        llvmPackages_latest.clang-unwrapped 
        llvmPackages_latest.libclang 
    ]; 
    buildInputs = with pkgs; [ 
        fuse 
    ]; 
    packages = [ 
        toolchain 
        build-musl
    ]; 
    RUST_SRC_PATH = "${toolchain}/lib/rustlib/src/rust/library"; 
    XWIN_CACHE_DIR = ".xwin-cache"; 
    LIBCLANG_PATH = "${pkgs.llvmPackages_latest.libclang.lib}/lib"; 
    
    CC_x86_64_pc_windows_msvc = "${pkgs.llvmPackages_latest.clang-unwrapped}/bin/clang-cl"; 
    CXX_x86_64_pc_windows_msvc = "${pkgs.llvmPackages_latest.clang-unwrapped}/bin/clang-cl"; 
    AR_x86_64_pc_windows_msvc = "${pkgs.llvmPackages_latest.llvm}/bin/llvm-lib";
    
    shellHook = ''
        build-win() {
            env NIX_CFLAGS_COMPILE="" NIX_LDFLAGS="" RUSTFLAGS="-C target-feature=+crt-static" cargo xwin build --release --target x86_64-pc-windows-msvc "$@"
        }
    '';
}

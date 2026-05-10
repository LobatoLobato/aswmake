let
  rust-overlay = builtins.fetchTarball "https://github.com/oxalica/rust-overlay/archive/master.tar.gz";
  pkgs = import <nixpkgs> {
    overlays = [(import rust-overlay)];
  };
  toolchain = pkgs.rust-bin.fromRustupToolchainFile ./toolchain.toml;
in
  pkgs.mkShell {
    nativeBuildInputs = with pkgs; [
      pkg-config
    ];

    buildInputs = with pkgs; [
      fuse
      # openssl
    ];
    packages = [
      toolchain
    ];

    RUST_SRC_PATH = "${toolchain}/lib/rustlib/src/rust/library";
  }
# shell.nix







# pkgs.mkShell {
#   nativeBuildInputs = with pkgs; [
#     pkg-config
#     cargo
#     rustc
#     (pkgs.rust-bin.stable.latest.default.override {
#       extensions = [ "rust-src" "rust-analyzer" ];
#     })
#   ];

#   buildInputs = with pkgs; [
#     fuse
#     # openssl
#   ];

#   # shellHook = ''
#   #   export PKG_CONFIG_PATH="${pkgs.openssl.dev}/lib/pkgconfig:$PKG_CONFIG_PATH"
#   # '';
# }

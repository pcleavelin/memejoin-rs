{
  inputs = {
    nixpkgs.url      = "github:nixos/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url  = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        local-rust = (pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain).override {
          extensions = [ "rust-analysis" ];
        };
      in
      {
        devShell = pkgs.mkShell {
          buildInputs = with pkgs; [
            local-rust
            rust-analyzer
            pkg-config
            gcc
            openssl
            python3
            ffmpeg
            cmake
            libopus
            youtube-dl
          ];
        };
      }
    );
}

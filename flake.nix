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

        packages = with pkgs; flake-utils.lib.flattenTree rec {
          default = rustPlatform.buildRustPackage rec {
            name = "memejoin-rs";
            version = "0.1.2-alpha";
            src = self;
            nativeBuildInputs = [ local-rust cmake gcc libopus ];

            cargoLock = {
              lockFile = ./Cargo.lock;
            };
          };

          docker = dockerTools.buildImage {
            name = "memejoin-rs";
            tag = "0.1.2-alpha";
            copyToRoot = buildEnv {
              name = "image-root";
              paths = [ default ffmpeg libopus youtube-dl ];
            };
            runAsRoot = ''
              #!${runtimeShell}
              mkdir -p /data
            '';
            config = {
              WorkingDir = "/data";
              Volumes = { "/data/config" = { }; "/data/sounds" = { }; };
              Entrypoint = [ "/bin/memejoin-rs" ];
            };
          };
        };
      }
    );
}

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
        yt-dlp = pkgs.yt-dlp.overrideAttrs (oldAttr: rec {
          inherit (oldAttr) name;
          version = "2023.02.17";
          src = pkgs.fetchFromGitHub {
            owner = "yt-dlp";
            repo = "yt-dlp";
            rev = "${version}";
            sha256 = "naC74T6aqCLX45wJLmygsMmTMqdqLbfXLjJKIKMRpiI=";
          };
        });
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
            pkg-config
            python3
            ffmpeg
            cmake
            libopus
            yt-dlp
          ];
        };

        packages = with pkgs; flake-utils.lib.flattenTree rec {
          default = rustPlatform.buildRustPackage rec {
            name = "memejoin-rs";
            version = "0.1.4-alpha";
            src = self;
            buildInputs = [ openssl.dev ];
            nativeBuildInputs = [ local-rust pkg-config openssl openssl.dev cmake gcc libopus ];

            cargoLock = {
              lockFile = ./Cargo.lock;
            };
          };

          docker = dockerTools.buildImage {
            name = "memejoin-rs";
            tag = "0.1.4-alpha";
            copyToRoot = buildEnv {
              name = "image-root";
              paths = [ default cacert openssl openssl.dev ffmpeg libopus youtube-dl yt-dlp ];
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

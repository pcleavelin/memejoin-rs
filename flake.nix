{
  inputs = {
    nixpkgs.url      = "github:nixos/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url  = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        tag = "v0.3.0-alpha";
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        yt-dlp = pkgs.yt-dlp.overrideAttrs (oldAttr: rec {
          inherit (oldAttr) name;
          version = "2025.09.26";
          src = pkgs.fetchFromGitHub {
            owner = "yt-dlp";
            repo = "yt-dlp";
            rev = "${version}";
            sha256 = "/uzs87Vw+aDNfIJVLOx3C8RyZvWLqjggmnjrOvUX1Eg=";
          };
        });
        local-rust = (pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain).override {
          extensions = [ "rust-analysis" ];
        };
      in
      {
        devShell = pkgs.mkShell {
          buildInputs = with pkgs; [
            git
            local-rust
            rust-analyzer
            pkg-config
            gcc
            openssl
            sqlite
            pkg-config
            python3
            ffmpeg
            cmake
            libopus
            yt-dlp
          ] ++ (if pkgs.system == "aarch64-darwin" || pkgs.system == "x86_64-darwin" then [ darwin.apple_sdk.frameworks.Security darwin.apple_sdk.frameworks.SystemConfiguration ] else []);
        };

        packages = with pkgs; flake-utils.lib.flattenTree rec {
          default = rustPlatform.buildRustPackage rec {
            inherit tag;
            name = "memejoin-rs";
            src = self;
            buildInputs = [ openssl.dev ];
            nativeBuildInputs = [ local-rust pkg-config openssl openssl.dev cmake gcc libopus sqlite ];

            cargoLock = {
              lockFile = ./Cargo.lock;
            };

            # lol, why does `buildRustPackage` not work without this?
            postPatch = ''
              ln -sf ${./Cargo.lock} Cargo.lock
            '';
          };

          docker = dockerTools.buildImage {
            inherit tag;
            name = "memejoin-rs";
            copyToRoot = buildEnv {
              name = "image-root";
              paths = [ default cacert openssl openssl.dev ffmpeg libopus yt-dlp sqlite ];
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

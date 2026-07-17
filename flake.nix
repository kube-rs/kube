{
  description = "Crate packaging and development environment for kube";

  inputs = {
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs =
    {
      self,
      crane,
      flake-utils,
      nixpkgs,
      rust-overlay,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        lib = pkgs.lib;

        version = "4.0.0";
        stableToolchainVersion = "1.88.0";
        certFile = "${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt";

        stableToolchain = pkgs.rust-bin.stable.${stableToolchainVersion}.default.override {
          extensions = [
            "clippy"
            "rust-analyzer"
            "rust-src"
            "rustfmt"
          ];
        };

        nightlyToolchain = pkgs.rust-bin.selectLatestNightlyWith (
          toolchain:
          toolchain.default.override {
            extensions = [
              "clippy"
              "rust-src"
              "rustfmt"
            ];
          }
        );

        craneLib = (crane.mkLib pkgs).overrideToolchain stableToolchain;
        nightlyCraneLib = (crane.mkLib pkgs).overrideToolchain nightlyToolchain;

        src = lib.cleanSourceWith {
          src = ./.;
          name = "kube-source";
          filter =
            path: type:
            (craneLib.filterCargoSources path type)
            || lib.hasSuffix ".json" path
            || lib.hasSuffix ".md" path
            || lib.hasSuffix ".stderr" path
            || lib.hasSuffix ".yaml" path
            || lib.hasSuffix ".yml" path;
        };

        cargoLockArgs = lib.optionalAttrs (builtins.pathExists ./Cargo.lock) {
          cargoLock = ./Cargo.lock;
        };

        nativeBuildInputs = [
          pkgs.cmake
          pkgs.perl
          pkgs.pkg-config
        ];

        buildInputs = [
          pkgs.openssl
        ]
        ++ lib.optionals pkgs.stdenv.isDarwin [
          pkgs.darwin.apple_sdk.frameworks.Security
          pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
        ];

        commonArgs = {
          inherit
            buildInputs
            nativeBuildInputs
            src
            version
            ;

          pname = "kube-workspace";
          strictDeps = true;
          cargoExtraArgs = "--locked";
          NIX_SSL_CERT_FILE = certFile;
          SSL_CERT_FILE = certFile;
        }
        // cargoLockArgs;

        cargoArtifacts = craneLib.buildDepsOnly (
          commonArgs
          // {
            cargoExtraArgs = "--locked --workspace --exclude kube-examples --exclude e2e";
          }
        );

        nightlyCargoArtifacts = nightlyCraneLib.buildDepsOnly (
          commonArgs
          // {
            cargoExtraArgs = "--locked --workspace --exclude kube-examples --exclude e2e";
          }
        );

        mkCrate =
          packageName:
          craneLib.buildPackage (
            commonArgs
            // {
              inherit cargoArtifacts;
              cargoExtraArgs = "--locked --package ${packageName}";
              doCheck = false;
              pname = packageName;
            }
          );

        ciApp = pkgs.writeShellApplication {
          name = "ci";
          runtimeInputs = [ pkgs.nix ];
          text = ''
            set -euo pipefail
            exec nix flake check --print-build-logs "$@"
          '';
        };

        generateLockfileApp = pkgs.writeShellApplication {
          name = "generate-lockfile";
          runtimeInputs = [
            stableToolchain
            pkgs.coreutils
            pkgs.gawk
            pkgs.git
          ];
          text = ''
            set -euo pipefail

            repo="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
            cd "$repo"

            if [ -f .gitignore ]; then
              tmp="$(mktemp)"
              awk '$0 != "Cargo.lock" { print }' .gitignore > "$tmp"
              mv "$tmp" .gitignore
            fi

            cargo generate-lockfile
          '';
        };

        installGitHooksApp = pkgs.writeShellApplication {
          name = "install-git-hooks";
          runtimeInputs = [
            pkgs.coreutils
            pkgs.git
          ];
          text = ''
            set -euo pipefail

            repo="$(git rev-parse --show-toplevel)"
            hooks_dir="$(git rev-parse --git-path hooks)"
            mkdir -p "$hooks_dir"

            for hook in pre-commit prepare-commit-msg post-commit; do
              src="$repo/scripts/git-hooks/$hook"
              dest="$hooks_dir/$hook"

              if [ ! -f "$src" ]; then
                printf 'missing hook script: %s\n' "$src" >&2
                exit 1
              fi

              current="$(readlink "$dest" 2>/dev/null || true)"
              if [ -e "$dest" ] && [ "$current" != "$src" ]; then
                backup="$dest.bak.$(date +%s)"
                mv "$dest" "$backup"
                printf 'backed up existing %s to %s\n' "$dest" "$backup"
              fi

              ln -sfn "$src" "$dest"
              printf 'installed %s -> %s\n' "$dest" "$src"
            done
          '';
        };

        cargo-fmt = nightlyCraneLib.cargoFmt {
          inherit src version;
          pname = "kube-workspace";
          doCheck = false;
        };
        cargo-fmt-apply = pkgs.writeShellApplication {
          name = "nightly-cargo-fmt";
          runtimeInputs = [ nightlyToolchain ];
          text = ''
            exec cargo fmt --all
          '';
        };

        # The justfile uses rustup-style cargo selectors; dispatch them to Nix toolchains.
        cargoToolchainShim = pkgs.writeShellApplication {
          name = "cargo";
          text = ''
            if [ "$#" -gt 0 ]; then
              case "$1" in
                +nightly)
                  shift
                  export PATH="${nightlyToolchain}/bin:$PATH"
                  exec "${nightlyToolchain}/bin/cargo" "$@"
                  ;;
                +stable|+${stableToolchainVersion})
                  shift
                  export PATH="${stableToolchain}/bin:$PATH"
                  exec "${stableToolchain}/bin/cargo" "$@"
                  ;;
              esac
            fi

            export PATH="${stableToolchain}/bin:$PATH"
            exec "${stableToolchain}/bin/cargo" "$@"
          '';
        };
      in
      {
        apps = {
          ci = {
            type = "app";
            program = "${ciApp}/bin/ci";
            meta.description = "Run the canonical Nix CI checks locally";
          };
          generate-lockfile = {
            type = "app";
            program = "${generateLockfileApp}/bin/generate-lockfile";
            meta.description = "Generate Cargo.lock and keep it unignored locally";
          };
          install-git-hooks = {
            type = "app";
            program = "${installGitHooksApp}/bin/install-git-hooks";
            meta.description = "Install local hooks that keep Cargo.lock out of commits";
          };
          fmt = {
            type = "app";
            program = "${cargo-fmt-apply}/bin/nightly-cargo-fmt";
            meta.description = "Run cargo fmt and apply the changes";
          };
          default = self.apps.${system}.ci;
        };

        packages = {
          default = self.packages.${system}.kube;
          kube = mkCrate "kube";
          kube-client = mkCrate "kube-client";
          kube-core = mkCrate "kube-core";
          kube-derive = mkCrate "kube-derive";
          kube-runtime = mkCrate "kube-runtime";
        };

        checks = {
          inherit (self.packages.${system})
            kube
            kube-client
            kube-core
            kube-derive
            kube-runtime
            ;
          inherit cargo-fmt;

          cargo-clippy = nightlyCraneLib.cargoClippy (
            commonArgs
            // {
              cargoArtifacts = nightlyCargoArtifacts;
              cargoClippyExtraArgs = "--workspace --exclude kube-examples --exclude e2e";
            }
          );

          cargo-test = craneLib.cargoTest (
            commonArgs
            // {
              inherit cargoArtifacts;
              cargoExtraArgs = "--locked --workspace --lib --exclude kube-examples --exclude e2e";
            }
          );

          cargo-doctest = craneLib.cargoDocTest (
            commonArgs
            // {
              inherit cargoArtifacts;
              cargoExtraArgs = "--locked --workspace --all-features --exclude kube-examples --exclude e2e";
            }
          );

          cargo-doc = nightlyCraneLib.cargoDoc (
            commonArgs
            // {
              cargoArtifacts = nightlyCargoArtifacts;
              cargoDocExtraArgs = "--workspace --all-features --no-deps --exclude kube-examples --exclude e2e";
              RUSTDOCFLAGS = "--cfg docsrs";
            }
          );
        };

        devShells.default = pkgs.mkShell {
          inputsFrom = [ self.packages.${system}.kube ];

          packages = [
            stableToolchain
            nightlyToolchain
            pkgs.cargo-deny
            pkgs.cargo-expand
            pkgs.cargo-hack
            pkgs.eza
            pkgs.fd
            pkgs.just
            pkgs.kubectl
            pkgs.ripgrep
          ];
          NIX_SSL_CERT_FILE = certFile;
          SSL_CERT_FILE = certFile;
          shellHook = ''
            export PATH="${cargoToolchainShim}/bin:$PATH"
            export RUST_SRC_PATH="${stableToolchain}/lib/rustlib/src/rust/library"
          '';
        };
      }
    );
}

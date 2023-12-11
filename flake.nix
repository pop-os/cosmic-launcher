{
  description = "Launcher for the COSMIC desktop environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    nix-filter.url = "github:numtide/nix-filter";
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, nix-filter, crane, fenix }:
    flake-utils.lib.eachSystem [ "x86_64-linux" "aarch64-linux" ] (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        craneLib = crane.lib.${system}.overrideToolchain fenix.packages.${system}.stable.toolchain;
        pkgDef = {
          src = nix-filter.lib.filter {
            root = ./.;
            exclude = [
              ./.gitignore
              ./flake.nix
              ./flake.lock
              ./LICENSE
              ./debian
            ];
          };
          nativeBuildInputs = with pkgs; [
            just
            pkg-config
            autoPatchelfHook
          ];
          buildInputs = with pkgs; [
            libxkbcommon
            wayland 
            freetype
            fontconfig
            expat
            lld
            desktop-file-utils
            stdenv.cc.cc.lib
            desktop-file-utils
           ];
          runtimeDependencies = with pkgs; [
            wayland
          ];
        };

        cargoArtifacts = craneLib.buildDepsOnly pkgDef;
        cosmic-launcher= craneLib.buildPackage (pkgDef // {
          inherit cargoArtifacts;
        });
      in {
        checks = {
          inherit cosmic-launcher;
        };

        packages.default = cosmic-launcher.overrideAttrs (oldAttrs: rec {
          buildPhase= ''
            just prefix=$out build-release
          '';
          installPhase = ''
            just prefix=$out install
          '';
        });

        apps.default = flake-utils.lib.mkApp {
          drv = cosmic-launcher;
        };

        devShells.default = pkgs.mkShell rec {
          inputsFrom = builtins.attrValues self.checks.${system};
          LD_LIBRARY_PATH = pkgs.lib.strings.makeLibraryPath (builtins.concatMap (d: d.runtimeDependencies) inputsFrom);
        };
      });

  nixConfig = {
    # Cache for the Rust toolchain in fenix
    extra-substituters = [ "https://nix-community.cachix.org" ];
    extra-trusted-public-keys = [ "nix-community.cachix.org-1:mB9FSh9qf2dCimDSUo8Zy7bkq5CX+/rkCWyvRCYg3Fs=" ];
  };
}

{
  inputs = {
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    utils = {
      url = "github:numtide/flake-utils";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      utils,
      fenix,
    }:
    utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        fenix' = pkgs.callPackage fenix { };
        rustPlatform = pkgs.makeRustPlatform {
          cargo = fenix'.latest.cargo;
          rustc = fenix'.latest.rustc;
        };
        crate = rustPlatform.buildRustPackage {
          name = "dbrunner";
          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
            allowBuiltinFetchGit = true;
          };
          useNextest = true;
          # unknown variant `2024`, expected one of `2015`, `2018`, `2021`
          auditable = false;

          nativeBuildInputs = [ pkgs.protobuf ];
          buildInputs = [ pkgs.libiconv ];
          strictDeps = true;
        };
      in
      {
        checks = {
          inherit crate;
        };
        packages.default = crate;

        packages.docker = pkgs.dockerTools.buildImage {
          name = "dbrunner";
          tag = "latest";

          copyToRoot = pkgs.buildEnv {
            name = "image-root";
            paths = [ crate ];
            pathsToLink = [ "/bin" ];
          };

          config = {
            Cmd = [ "/bin/dbrunner" ];
            ExposedPorts = {
              "8080/tcp" = { };
            };
            Env = [ "ADDR=0.0.0.0:8080" ];
          };
        };
      }
    );
}

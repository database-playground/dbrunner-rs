{
  inputs = {
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane = {
      url = "github:ipetkov/crane";
      inputs = {
        nixpkgs.follows = "nixpkgs";
      };
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
      crane,
      fenix,
    }:
    utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        fenix' = pkgs.callPackage fenix { };
        crane' = (crane.mkLib pkgs).overrideToolchain (
          (fenix'.combine [
            fenix'.latest.cargo
            fenix'.latest.rustc
          ])
        );
        crate = crane'.buildPackage {
          src = ./.;
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
            Env = [ "ADDR=0.0.0.0:3000" ];
          };
        };
      }
    );
}

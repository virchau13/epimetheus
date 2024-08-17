{
  description = "Discord bot";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, crane, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};

        craneLib = crane.mkLib pkgs;

        epimetheus = craneLib.buildPackage {
          pname = "epimetheus";
          version = "0.1.0";

          src = craneLib.cleanCargoSource ./.;
          strictDeps = true;

          buildInputs = with pkgs; [
            pkg-config
            m4
            aflplusplus
          ];
        };
      in
      {
        checks = { inherit epimetheus; };

        packages.default = epimetheus;

        apps.default = flake-utils.lib.mkApp {
          drv = epimetheus;
        };

        devShells.default = craneLib.devShell {
          checks = self.checks.${system};
          packages = epimetheus.buildInputs;
          LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath epimetheus.buildInputs}";
        };
      });
}

{
  description = "A Discord bot";

  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";
    utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, utils }: utils.lib.eachDefaultSystem(system: let
    pkgs = import nixpkgs { inherit system; };
  in {
    packages = rec {
      epimetheus = with pkgs; rustPlatform.buildRustPackage  {
        pname = "epimetheus";
        version = "0.1.0";

        src = ./.;

        buildInputs = [ m4 ];
        nativeBuildInputs = [ m4 ];

        cargoLock = {
          lockFile = ./Cargo.lock;
        };
      };
      default = epimetheus;
    };
    devShell = with pkgs; stdenv.mkDerivation {
      name = "epimetheus";
      buildInputs = [ m4 ];
    };
  });
}

{
  description = "A Discord bot";

  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";
    utils.url = "github:numtide/flake-utils";
    fenix.url = "github:nix-community/fenix/monthly";
  };

  outputs = { self, nixpkgs, utils, fenix }: utils.lib.eachDefaultSystem(system: let
    pkgs = import nixpkgs { inherit system; };
    deps = with pkgs; [ m4 aflplusplus pkg-config ];
    cargo-afl = pkgs.callPackage ./cargo-afl.nix {};
    devDeps = deps; # ++ [ cargo-afl ];
  in {
    packages = rec {
      epimetheus = with pkgs; rustPlatform.buildRustPackage  {
        pname = "epimetheus";
        version = "0.1.0";

        src = ./.;

        buildInputs = deps;
        nativeBuildInputs = deps;

        cargoLock = {
          lockFile = ./Cargo.lock;
        };
      };
      default = epimetheus;
    };
    devShells = {
      default = with pkgs; mkShell {
        buildInputs = devDeps;
      };
      nightly = with pkgs; mkShell {
        buildInputs = devDeps ++ [ fenix.packages.${system}.default.toolchain ];
      };
    };
  });
}

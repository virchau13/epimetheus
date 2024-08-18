{ rustPlatform, fetchFromGitHub }:

rustPlatform.buildRustPackage {
  pname = "cargo-afl";
  version = "0.15.10";

  src = fetchFromGitHub {
    owner = "rust-fuzz";
    repo = "afl.rs";
    rev = "e50808527785b31cb2168c77b070e0e5d00d8e58";
    hash = "sha256-iwwnXd2/sLFQwKHzmWZyE15qVm7DwgTaTS5aKfzvzEI=";
  };

  cargoHash = "sha256-dfZ4K9osrpshrLS/xXV4uW6S5Xc7TLRiZFsy/v56Agw=";

  cargoBuildFlags = [ "-p" "cargo-afl" ];
  cargoTestFlags = [ "-p" "cargo-afl" ];

  preCheck = ''
    mkdir $TMPDIR/data
    export XDG_DATA_HOME=$TMPDIR/data
  '';
}

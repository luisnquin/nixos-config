{
  fetchFromGitHub,
  rustPlatform,
}:
rustPlatform.buildRustPackage rec {
  pname = "netscanner";
  version = "0.4.0";

  src = fetchFromGitHub {
    owner = "Chleba";
    repo = pname;
    rev = "v${version}";
    hash = "sha256-p5FG3srQJA+siFfrVamSTG7rwsTmSTQj0+qBwzl0xog=";
  };

  cargoSha256 = "sha256-GW40bIR7VfozlOI3Su1SmLn6ZL9Pf0uGvD0EFLbQ6rg=";
}

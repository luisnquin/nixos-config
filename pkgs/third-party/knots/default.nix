{
  fetchFromGitHub,
  rustPlatform,
}:
rustPlatform.buildRustPackage rec {
  pname = "knots";
  version = "1.17.0";

  src = fetchFromGitHub {
    owner = "brandon-arrendondo";
    repo = "knots";
    tag = "v${version}";
    hash = "sha256-GyEIDTlkSieIfOeApJmQBJ9syiSNXuYjmHJ+FpDmjo4=";
  };

  patches = [./staged.patch];

  cargoHash = "sha256-gJGX76WjF46AsVOy/g/jzp7Dhjdt2P/SKZwllGIX/R0=";

  meta.mainProgram = "knots";
}

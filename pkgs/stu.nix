{
  fetchFromGitHub,
  rustPlatform,
}:
rustPlatform.buildRustPackage rec {
  pname = "s3-tui";
  version = "unstable";

  src = fetchFromGitHub {
    owner = "lusingander";
    repo = "stu";
    rev = "2ba7aa2810cb38f41b781362f6185d38171cf198";
    hash = "sha256-O1dMStijhkE+nlVkABnL9FFZgTIa94ZZzbpL8BMG0mY=";
  };

  cargoSha256 = "sha256-WHjfiV1uyF1GSs1Detga5mpNBL3RvpJfYEOVfsAmruQ=";
}

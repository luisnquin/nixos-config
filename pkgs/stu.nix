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
    rev = "94b919c7f989150c6b68fc76d2e70375d8a35ba0";
    hash = "sha256-gB9jYJSuDtqt5EJdqSOo4Jz0j8Ei9eBKeoYdaA389s0=";
  };

  cargoSha256 = "sha256-WHjfiV1uyF1GSs1Detga5mpNBL3RvpJfYEOVfsAmruQ=";
}

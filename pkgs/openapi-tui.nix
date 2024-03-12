{
  fetchFromGitHub,
  rustPlatform,
  ...
}:
rustPlatform.buildRustPackage rec {
  pname = "openapi-tui";
  version = "0.4.0";

  src = fetchFromGitHub {
    owner = "zaghaghi";
    repo = "openapi-tui";
    rev = version;
    hash = "sha256-7xkjlX3+/hdVN2PXoiXbouSoMLy0Qe8uMRlPHWJO5Ts=";
  };

  cargoSha256 = "sha256-U8TOms8C7vV64OKKdJhMAoOha9s2lBqfBWU7pyZ0h/s=";
}

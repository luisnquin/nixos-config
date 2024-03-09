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
    rev = "251e982b9d4e4c2a2a3c13bfb15328a1efa29d18";
    hash = "sha256-KMXDHYoNZovFbaEppP1fqNBNCyN2JAUomGVezXJxAAk=";
  };

  cargoSha256 = "sha256-tObPaRtNLekKWgHXijUIHDn9DWNPW2UjqezjJAxvoy4=";
}

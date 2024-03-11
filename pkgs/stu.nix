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
    rev = "cc93f7325ba8ef75f741ae3fd91d8ea2ea161155";
    hash = "sha256-2BS5hmAQIiFf9Lr/CZCF0VoOjptWF6QyfxK3vCkEGfc=";
  };

  cargoSha256 = "sha256-tObPaRtNLekKWgHXijUIHDn9DWNPW2UjqezjJAxvoy4=";
}

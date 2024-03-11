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
    rev = "b45af223371e54a81ea1b04f31c6545c5ca80f7d";
    hash = "sha256-32EwxLzZ9LVtNGAl5FyA2b5lEa10ebGzPNOJ/vK0Kr0=";
  };

  cargoSha256 = "sha256-tObPaRtNLekKWgHXijUIHDn9DWNPW2UjqezjJAxvoy4=";
}

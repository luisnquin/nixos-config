{pkgs ? import <nixpkgs> {}}:
pkgs.buildGoModule rec {
  pname = "github-tui";
  version = "unstable";
  src = pkgs.fetchFromGitHub {
    name = "${pname}-source";
    owner = "skanehira";
    repo = "github-tui";
    rev = "ecc4a3c5953c5cc2b8cac0c9fb308033ae7396f9";
    hash = "sha256-c4mrGP9phHlCvg4YuQmWOWVzwtqKhP53vWsfYkrLkFo=";
  };

  buildTarget = "./cmd/ght";

  vendorHash = "sha256-ph4Fb4dvI6htw6i9ypmhu1YMUi054+KkGkPz5CI/aA4=";
  doCheck = true;
}

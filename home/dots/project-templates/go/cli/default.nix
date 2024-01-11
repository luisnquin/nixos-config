{pkgs ? import <nixpkgs> {}}:
pkgs.buildGoModule rec {
  pname = "go-project";
  version = "unstable";

  src = builtins.path {
    name = pname;
    path = ./.;
  };
  
  vendorHash = null;

  ldflags = ["-X main.version=${version}"];
  buildTarget = ".";
}

{pkgs ? import <nixpkgs> {}}:
pkgs.buildGoModule rec {
  pname = "go-project";
  version = "unstable";

  src = builtins.path {
    name = pname;
    path = ./.;
  };
  
  vendorHash = "sha256-bGDhws+Ye/VDqIAcfBTIkjfp3IWkEx9a/Xwri0wl265=";

  ldflags = ["-X main.version=${version}"];
  buildTarget = ".";
}

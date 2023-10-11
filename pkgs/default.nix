{pkgs ? import <nixpkgs> {}, ...}:
with pkgs.lib; let
  packages = {
    transg-tui = ./transg-tui.nix;
    pg-ping = ./pg-ping.nix;
    minecraft = ./minecraft;
    pp = ./panicparse.nix;
    npkill = ./npkill.nix;
    no = ./no.nix;
  };
in
  attrsets.mapAttrs (_name: packagePath: pkgs.callPackage packagePath {}) packages

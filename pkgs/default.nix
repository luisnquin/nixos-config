{pkgs ? import <nixpkgs> {}, ...}:
with pkgs.lib; let
  packages = {
    panicparse = ./panicparse.nix;
    transg-tui = ./transg-tui.nix;
    minecraft = ./minecraft.nix;
    pg-ping = ./pg-ping.nix;
    npkill = ./npkill.nix;
    no = ./no.nix;
  };
in
  attrsets.mapAttrs (_name: packagePath: pkgs.callPackage packagePath {}) packages

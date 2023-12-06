{pkgs ? import <nixpkgs> {}, ...}:
with pkgs.lib; let
  packages = {
    coders-crux = ./coders-crux.nix;
    transg-tui = ./transg-tui.nix;
    pg-ping = ./pg-ping.nix;
    # minecraft = ./minecraft;
    pp = ./panicparse.nix;
    npkill = ./npkill.nix;
    no = ./no.nix;
  };
in
  attrsets.mapAttrs (_name: packagePath: pkgs.callPackage packagePath {}) packages

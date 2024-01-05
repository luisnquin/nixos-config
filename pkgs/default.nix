{pkgs ? import <nixpkgs> {}, ...}:
with pkgs.lib; let
  packages = {
    vimix-cursor-theme = ./vimix-cursor-theme.nix;
    coders-crux = ./coders-crux.nix;
    transg-tui = ./transg-tui.nix;
    pg-ping = ./pg-ping.nix;
    netflix = ./netflix.nix;
    # minecraft = ./minecraft;
    npkill = ./npkill.nix;
    tpm = ./tpm.nix;
    no = ./no.nix;
  };
in
  attrsets.mapAttrs (_name: packagePath: pkgs.callPackage packagePath {}) packages

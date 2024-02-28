{pkgs ? import <nixpkgs> {}, ...}:
with pkgs.lib; let
  packages = {
    vimix-cursor-theme = ./vimix-cursor-theme.nix;
    coders-crux = ./coders-crux.nix;
    transg-tui = ./transg-tui.nix;
    emoji-fzf = ./emoji-fzf.nix;
    ecsview = ./ecsview.nix;
    pg-ping = ./pg-ping.nix;
    netflix = ./netflix.nix;
    yaak = ./yaak.nix;
    stu = ./stu.nix;
    # minecraft = ./minecraft;
    npkill = ./npkill.nix;
    tpm = ./tpm.nix;
    ght = ./ght.nix;
    no = ./no.nix;
  };
in
  attrsets.mapAttrs (_name: packagePath: pkgs.callPackage packagePath {}) packages

{pkgs ? import <nixpkgs> {}, ...}:
with pkgs.lib; let
  packages = {
    dbeaver = ./dbeaver.nix;
    pg-ping = ./pg-ping.nix;
    netflix = ./netflix.nix;
    tpm = ./tpm.nix;
  };
in
  attrsets.mapAttrs (_name: packagePath: pkgs.callPackage packagePath {}) packages

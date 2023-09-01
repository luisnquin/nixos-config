{pkgs ? import <nixpkgs> {}, ...}: {
  environment.systemPackages = with pkgs; let
    paths = [
      ./panicparse.nix
      ./transg-tui.nix
      ./minecraft.nix
      ./pg-ping.nix
      ./npkill.nix
      ./no.nix
    ];
  in
    lib.lists.forEach paths (path: callPackage path {});
}

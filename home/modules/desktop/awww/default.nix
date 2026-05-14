{
  pkgs,
  lib,
  libx,
  config,
  ...
}: let
  wallpaperFiles = libx.fs.getFilesInDirectory ./wallpapers;
  inherit (pkgs.scripts) awww-switcher;
  wallpaperArgs = builtins.concatStringsSep " " (map builtins.toString (wallpaperFiles ++ [./background.gif]));
in {
  services.awww.enable = true;

  wayland.windowManager.hyprland.extraConfig = lib.mkIf config.wayland.windowManager.hyprland.enable ''
    hl.bind("SUPER + L", hl.dsp.exec_cmd("${awww-switcher}/bin/cli ${wallpaperArgs}"))
  '';
}

{
  pkgs,
  libx,
  ...
}: {
  services.awww.enable = true;

  xdg.configFile = let
    wallpaperFiles = libx.fs.getFilesInDirectory ./wallpapers;

    inherit (pkgs.scripts) swww-switcher;
  in {
    "hypr/hyprland.conf".text = ''
      bind = $mainMod, L, exec, ${swww-switcher}/bin/cli ${builtins.concatStringsSep " " (wallpaperFiles
        ++ [
          ./background.gif
        ])}
    '';
  };
}

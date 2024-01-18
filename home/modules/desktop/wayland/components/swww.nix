{
  swww-switcher,
  pkgs,
  libx,
  ...
}: {
  home.packages = [pkgs.swww];

  xdg.configFile = let
    wallpaperFiles = libx.fs.getFilesInDirectory ../../../../dots/wallpapers;
    swww-switcher-bin = "${swww-switcher}/bin/cli";
  in {
    "hypr/hyprland.conf".text = ''
      exec-once = ${pkgs.swww}/bin/swww init
      exec = ${pkgs.swww}/bin/swww img ${../../../../dots/background.gif}

      bind = $mainMod, L, exec, ${swww-switcher-bin} ${builtins.concatStringsSep " " (wallpaperFiles
        ++ [
          ../../../../dots/background.gif
        ])}
    '';
  };
}

{
  system,
  inputs,
  libx,
  ...
}: {
  services.swww.enable = true;

  xdg.configFile = let
    wallpaperFiles = libx.fs.getFilesInDirectory ./wallpapers;

    inherit (inputs.nix-scripts.packages.${system}) swww-switcher;
  in {
    "hypr/hyprland.conf".text = ''
      bind = $mainMod, L, exec, ${swww-switcher}/bin/cli ${builtins.concatStringsSep " " (wallpaperFiles
        ++ [
          ./background.gif
        ])}
    '';
  };
}

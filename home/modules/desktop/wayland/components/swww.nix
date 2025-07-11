{
  inputs,
  system,
  pkgs,
  ...
}: {
  services.swww.enable = true;

  xdg.configFile = let
    wallpaperFiles = pkgs.libx.fs.getFilesInDirectory ../../../../dots/wallpapers;

    inherit (inputs.nix-scripts.packages.${system}) swww-switcher;
  in {
    "hypr/hyprland.conf".text = ''
      bind = $mainMod, L, exec, ${swww-switcher}/bin/cli ${builtins.concatStringsSep " " (wallpaperFiles
        ++ [
          ../../../../dots/background.gif
        ])}
    '';
  };
}

{pkgs, ...}: let
  inherit (pkgs) callPackage;

  getFolderPaths = with builtins;
    folderPath: let
      fileNames = attrNames (readDir folderPath);
    in
      map (p: folderPath + ("/" + p)) fileNames;

  wallpaperFiles = getFolderPaths ./../../../dots/wallpapers;
in {
  home.packages = [pkgs.swww];

  xdg.configFile."hypr/hyprland.conf".text = let
    swww-switcher-bin = "${callPackage ./../../../scripts/swww-switcher {}}/bin/cli";
  in ''
    exec-once = ${pkgs.swww}/bin/swww init
    exec = ${pkgs.swww}/bin/swww img ${./../../../dots/background.png}

    bind = $mainMod, L, exec, ${swww-switcher-bin} ${builtins.concatStringsSep " " (wallpaperFiles
      ++ [
        ./../../../dots/background.png
        ./../../../dots/background.gif
      ])}
  '';
}

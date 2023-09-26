{pkgs, ...}: let
  inherit (pkgs) callPackage;

  files = builtins.attrNames (builtins.readDir ./../../../dots/wallpapers);

  wallpaper-files = builtins.map (p: ./. + "/../../../dots/wallpapers" + ("/" + p)) files;
in {
  home.packages = with pkgs; [
    swww
  ];

  xdg.configFile."hypr/hyprland.conf".text = let
    swww-switcher-bin = "${callPackage ./../../../scripts/swww-switcher {}}/bin/cli";
  in ''
    exec-once = ${pkgs.swww}/bin/swww init
    exec = ${pkgs.swww}/bin/swww img ${./../../../dots/background.png}

    bind = $mainMod, L, exec, ${swww-switcher-bin} ${builtins.concatStringsSep " " (wallpaper-files
      ++ [
        ./../../../dots/background.png
        ./../../../dots/background.gif
      ])}
  '';
}

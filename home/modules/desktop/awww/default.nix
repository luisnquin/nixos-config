{
  pkgs,
  lib,
  config,
  ...
}: let
  cfg = config.services.awww;
  inherit (pkgs.scripts) awww-switcher;

  wallpapersPkg =
    pkgs.runCommand "hypr-wallpapers" {
      preferLocalBuild = true;
    } ''
      mkdir -p $out
      cp -r ${./wallpapers}/. $out/
      cp ${./background.gif} $out/
    '';

  wallpaperNames = lib.naturalSort (
    lib.attrNames (lib.filterAttrs (_: t: t == "regular") (builtins.readDir ./wallpapers))
  );

  wallpaperFiles = map (name: "${wallpapersPkg}/${name}") wallpaperNames;
  backgroundFile = "${wallpapersPkg}/background.gif";
  defaultWallpaper = lib.head wallpaperFiles;

  wallpaperArgs = lib.concatStringsSep " " (map lib.escapeShellArg (wallpaperFiles ++ [backgroundFile]));
  awwwImg = "${lib.getExe' cfg.package "awww"} img ${lib.escapeShellArg defaultWallpaper}";
in {
  services.awww.enable = true;

  wayland.windowManager.hyprland.extraConfig = lib.mkIf config.wayland.windowManager.hyprland.enable ''
    hl.on("hyprland.start", function()
      hl.exec_cmd("${awwwImg}")
    end)

    hl.bind("SUPER + L", hl.dsp.exec_cmd("${awww-switcher}/bin/cli ${wallpaperArgs}"))
  '';
}

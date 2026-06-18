{
  pkgs,
  lib,
  config,
  ...
}: let
  cfg = config.services.awww;

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

  switchWallpaper = pkgs.writers.writePython3Bin "awww-switch-wallpaper" {} ''
    from pathlib import Path
    import subprocess
    import sys


    AWWW = "${lib.getExe' cfg.package "awww"}"


    def current_wallpaper_name() -> str:
        output = subprocess.check_output([AWWW, "query"], text=True)
        lines = [line for line in output.splitlines() if line]

        if not lines:
            return ""

        current_path = lines[-1].split()[-1]
        return Path(current_path).name


    def next_wallpaper(wallpapers: list[str]) -> str:
        current_name = current_wallpaper_name()

        for index, wallpaper in enumerate(wallpapers):
            if Path(wallpaper).name == current_name:
                return wallpapers[(index + 1) % len(wallpapers)]

        return wallpapers[0]


    def main() -> int:
        wallpapers = sys.argv[1:]

        if not wallpapers:
            print("no wallpapers have been specified", file=sys.stderr)
            return 1

        subprocess.run([AWWW, "img", next_wallpaper(wallpapers)], check=True)
        return 0


    if __name__ == "__main__":
        raise SystemExit(main())
  '';

  wallpaperArgs = lib.concatStringsSep " " (map lib.escapeShellArg wallpaperFiles);
  awwwImg = "${lib.getExe' cfg.package "awww"} img ${lib.escapeShellArg defaultWallpaper}";
in {
  services.awww.enable = true;

  wayland.windowManager.hyprland.extraConfig = lib.mkIf config.wayland.windowManager.hyprland.enable ''
    hl.on("hyprland.start", function()
      hl.exec_cmd("${awwwImg}")
    end)

    hl.bind("SUPER + L", hl.dsp.exec_cmd("${lib.getExe switchWallpaper} ${wallpaperArgs}"))
  '';
}

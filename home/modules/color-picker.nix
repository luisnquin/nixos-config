{
  isWayland,
  pkgs,
  lib,
  ...
}: let
  package =
    if isWayland
    then pkgs.wl-color-picker
    else pkgs.xcolor;
in {
  home = {
    packages = [package];

    shellAliases = {
      color-picker = "${lib.getExe package}";
    };
  };
}

{
  isWayland,
  pkgs,
  lib,
  ...
}: let
  package = let
    color-picker =
      if isWayland
      then {
        package = pkgs.wl-color-picker;
        extraArgs = "";
      }
      else {
        package = pkgs.xcolor;
        extraArgs = "-s";
      };
  in
    color-picker.package.overrideAttrs (
      old: {
        nativeBuildInputs = old.nativeBuildInputs ++ [pkgs.copyDesktopItems];

        desktopItems = [
          (pkgs.makeDesktopItem rec {
            name = "Color Picker";
            exec = "${lib.getExe color-picker.package} ${color-picker.extraArgs}";
            icon = name;
            desktopName = name;
            genericName = let
              pname =
                if builtins.isString old.pname
                then old.pname
                else old.name;
            in "${pname} - ${old.meta.description}";
          })
        ];
      }
    );
in {
  home = {
    packages = [package];

    shellAliases = {
      "color-picker" = "${lib.getExe package}";
    };
  };
}

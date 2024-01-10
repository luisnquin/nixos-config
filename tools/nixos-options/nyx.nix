{
  config,
  pkgs,
  lib,
  ...
}:
with lib; let
  cfg = config.tools.nyx;
in {
  options = {
    tools.nyx = {
      enable = mkEnableOption "nyx";

      hyprlandSupport = mkOption {
        type = types.bool;
        default = config.programs.hyprland.enable ? false;
      };

      notificationIcon = mkOption {
        type = types.nullOr types.bool;
        default = null;
      };

      dotfilesDir = mkOption {
        type = types.nullOr types.str;
        default = null;
      };
    };
  };

  config = mkIf cfg.enable {
    environment.systemPackages = let
      nyx = pkgs.callPackage ./../nyx (
        lib.filterAttrs (n: v: n != "enable" && v != null) cfg
      );
    in [nyx];
  };
}
# builtins.removeAttrs cfg [ "enable" ]


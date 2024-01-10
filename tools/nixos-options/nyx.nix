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
    };
  };

  config = mkIf cfg.enable {
    environment.systemPackages = let
      nyx = pkgs.callPackage ./../nyx (
        {
          inherit (cfg) hyprlandSupport;
        }
        // (
          if cfg.notificationIcon != null
          then {
            inherit (cfg) notificationIcon;
          }
          else {}
        )
      );
    in [nyx];
  };
}

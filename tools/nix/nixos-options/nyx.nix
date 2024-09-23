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

      notificationIcon = mkOption {
        type = types.nullOr types.bool;
        default = null;
      };

      dotfilesDir = mkOption {
        type = types.nullOr types.str;
        default = "$HOME/.dotfiles";
      };
    };
  };

  config = mkIf cfg.enable {
    environment = {
      systemPackages = let
        nyx = pkgs.callPackage ../../nyx (
          lib.filterAttrs (n: v: !(lib.elem n ["enable" "dotfilesDir"]) && v != null) cfg
        );
      in [nyx];

      variables = {
        "DOTFILES_PATH" = cfg.dotfilesDir;
      };
    };
  };
}

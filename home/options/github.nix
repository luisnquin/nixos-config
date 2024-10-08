{
  config,
  pkgs,
  lib,
  ...
}:
with lib; let
  cfg = config.programs.github-tools;
in {
  options = {
    programs.github-tools = {
      enable = mkEnableOption "github";

      act = mkOption {
        type = types.bool;
        default = true;
      };

      cli = mkOption {
        type = types.bool;
        default = true;
      };

      tui = mkOption {
        type = types.bool;
        default = true;
      };
    };
  };

  config = mkIf cfg.enable {
    home = {
      packages =
        [
          pkgs.hub
        ]
        ++ (
          if cfg.act
          then [pkgs.act]
          else []
        )
        ++ (
          if cfg.tui
          then [pkgs.extra.ght]
          else []
        )
        ++ (
          if cfg.cli
          then [pkgs.gh]
          else []
        );

      shellAliases = {
        git = "hub";
      };
    };
  };
}

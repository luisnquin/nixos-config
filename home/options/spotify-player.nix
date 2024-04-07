{
  config,
  pkgs,
  lib,
  ...
}:
with lib; let
  cfg = config.programs.spotify-player;
in {
  options = {
    programs.spotify-player = {
      enable = mkEnableOption "spotify-player";
      package = mkOption {
        type = types.package;
        default = pkgs.spotify-player;
      };

      alias = mkOption {
        type = types.str;
        default = "spotify-player";
      };
    };
  };

  config = mkIf cfg.enable {
    home = {
      packages = [cfg.package];

      shellAliases = mkIf (cfg.alias != "") {
        ${cfg.alias} = lib.getExe cfg.package;
      };
    };

    # xdg.configFile = mkIf (cfg.settings != null) {
    #   "spotify-tui/config.yml".source = (pkgs.formats.yaml {}).generate "spotify-tui-yml-config" cfg.settings;
    # };
  };
}

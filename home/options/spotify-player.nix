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
    };
  };

  config = mkIf cfg.enable {
    home.packages = [
      cfg.package
    ];

    # xdg.configFile = mkIf (cfg.settings != null) {
    #   "spotify-tui/config.yml".source = (pkgs.formats.yaml {}).generate "spotify-tui-yml-config" cfg.settings;
    # };
  };
}

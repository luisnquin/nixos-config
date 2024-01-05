{
  config,
  pkgs,
  lib,
  ...
}:
with lib; let
  cfg = config.programs.lazydocker;
in {
  options = {
    programs.lazydocker = {
      enable = mkEnableOption "lazydocker";
      config = mkOption {
        default = {};
        type = types.attrs;
      };
    };
  };

  config = mkIf cfg.enable {
    home.packages = [
      pkgs.lazydocker
    ];

    xdg.configFile = {
      "lazydocker/config.yml".source = (pkgs.formats.yaml {}).generate "lazydocker-config" cfg.config;
    };
  };
}

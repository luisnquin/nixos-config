{
  config,
  pkgs,
  lib,
  ...
}:
with lib; let
  cfg = config.tools.tplr;
in {
  options = {
    tools.tplr = {
      enable = mkEnableOption "nyx";

      templates = mkOption {
        type = types.attrs;
        default = {};
      };
    };
  };

  config = mkIf cfg.enable {
    home.packages = [
      (pkgs.callPackage ../../tplr {})
    ];

    xdg.configFile = {
      "tplr/tree.json".source = (pkgs.formats.json {}).generate "tplr-tree" cfg.templates;
    };
  };
}

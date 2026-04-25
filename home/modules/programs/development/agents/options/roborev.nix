{
  config,
  inputs,
  pkgs,
  lib,
  ...
}: let
  inherit (lib) mkOption types;

  tomlFormat = pkgs.formats.toml {};
  cfg = config.programs.roborev;
in {
  options = {
    programs.roborev = {
      enable = mkOption {
        type = types.bool;
        default = true;
        description = "Whether to install and configure Roborev.";
      };

      package = mkOption {
        type = types.package;
        default = inputs.roborev.packages.${pkgs.stdenv.hostPlatform.system}.default;
        description = "Roborev package to install.";
      };

      settings = mkOption {
        type = tomlFormat.type;
        default = {};
        description = "Roborev configuration written to ~/.roborev/config.toml.";
      };
    };
  };

  config = lib.mkIf cfg.enable {
    home.packages = [cfg.package];

    home.file.".roborev/config.toml".source = tomlFormat.generate "roborev-config.toml" cfg.settings;
  };
}

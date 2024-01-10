{
  config,
  pkgs,
  lib,
  ...
}:
with lib; let
  cfg = config.programs.act;
in {
  options = {
    programs.act = {
      enable = mkEnableOption "discord";
    };
  };

  config = mkIf cfg.enable {
    home.packages = [
      pkgs.act
    ];
  };
}

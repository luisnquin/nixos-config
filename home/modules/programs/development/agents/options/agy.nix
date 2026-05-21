{
  config,
  lib,
  pkgs,
  ...
}: let
  cfg = config.programs.agy;
in {
  options.programs.agy = {
    enable = lib.mkEnableOption "Agy, agentic coding CLI";

    package = lib.mkPackageOption pkgs "antigravity" {nullable = true;};
  };

  config = lib.mkIf cfg.enable {
    home.packages = lib.mkIf (cfg.package != null) [cfg.package];
  };
}

{
  config,
  lib,
  pkgs,
  ...
}: let
  cfg = config.programs.ee-workbench;

  inherit (lib) mkEnableOption mkIf mkOption mkPackageOption types;
in {
  options.programs.ee-workbench = {
    enable = mkEnableOption "ee-workbench, a git-backed electronics workbench";

    package = mkPackageOption pkgs "ee-workbench" {};

    dataDir = mkOption {
      type = types.str;
      default = "";
      example = "/home/user/Documents/workbench";
      description = ''
        Workbench repository, exported as `EE_WORKBENCH_DATA`. Empty keeps the
        default `$XDG_DATA_HOME/ee-workbench`. The repository is the storage:
        the CLI never commits, pulls or pushes on its own, so moving it here
        also moves what `ee git` operates on.
      '';
    };

    cad.enable = mkEnableOption ''
      `ee-freecad-server`, the native FreeCAD session `ee mechanical` drives.
      It links the installed FreeCAD and has to be started by hand; nothing
      spawns it, so a stray session can never hold a document open unnoticed
    '';
  };

  config = mkIf cfg.enable {
    home.packages = [cfg.package] ++ lib.optional cfg.cad.enable cfg.package.cad;

    home.sessionVariables = mkIf (cfg.dataDir != "") {
      EE_WORKBENCH_DATA = cfg.dataDir;
    };
  };
}

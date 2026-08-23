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
      The CLI starts it on the first command that needs it and it retires
      itself once idle, so there is nothing to launch and nothing to remember
      to stop
    '';

    cad.idleTimeout = mkOption {
      type = types.ints.unsigned;
      default = 900;
      description = ''
        Seconds an idle session waits before exiting, as `EE_WORKBENCH_CAD_IDLE`.
        A session holding a document nobody saved ignores this and stays up, so
        the timeout costs memory, never work. 0 keeps it running forever.
      '';
    };
  };

  config = mkIf cfg.enable {
    home.packages = [cfg.package] ++ lib.optional cfg.cad.enable cfg.package.cad;

    home.sessionVariables =
      lib.optionalAttrs (cfg.dataDir != "") {
        EE_WORKBENCH_DATA = cfg.dataDir;
      }
      # Pinned rather than left to PATH: `ee` and the server share a wire
      # protocol version and refuse each other across a mismatch.
      // lib.optionalAttrs cfg.cad.enable {
        EE_WORKBENCH_CAD_SERVER = lib.getExe cfg.package.cad;
        EE_WORKBENCH_CAD_IDLE = toString cfg.cad.idleTimeout;
      };
  };
}

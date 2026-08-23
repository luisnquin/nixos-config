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
    # `withCad` is the same `ee` behind a wrapper that names the server by store
    # path, so the profile holds one closure instead of two packages that a
    # session variable was supposed to keep in step. It did not: the variable is
    # only exported at login, so a shell that outlived the switch went on
    # spawning the server from the generation it was born in. The wrapper
    # overrides that, and `nix-store -q --references` proves the pairing.
    # Overriding it back is what the unwrapped `cfg.package` is for.
    home.packages =
      if cfg.cad.enable
      then [cfg.package.withCad cfg.package.cad]
      else [cfg.package];

    home.sessionVariables =
      lib.optionalAttrs (cfg.dataDir != "") {
        EE_WORKBENCH_DATA = cfg.dataDir;
      }
      # A preference, not a pin: nothing breaks if a shell carries an older one.
      // lib.optionalAttrs cfg.cad.enable {
        EE_WORKBENCH_CAD_IDLE = toString cfg.cad.idleTimeout;
      };
  };
}

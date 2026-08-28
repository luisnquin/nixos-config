{
  config,
  lib,
  pkgs,
  ...
}: let
  inherit (lib) mkEnableOption mkIf mkOption types;
  cfg = config.services.heft;

  scan = args: "${lib.getExe pkgs.heft} scan ${args}";
in {
  options.services.heft = {
    enable = mkEnableOption "disk census and dashboard";

    interval = mkOption {
      type = types.str;
      default = "daily";
      description = "OnCalendar expression for the regular census.";
    };

    deepInterval = mkOption {
      type = types.str;
      default = "weekly";
      description = ''
        OnCalendar expression for the deep census, which walks the whole nix
        store to re-measure its hardlink dedup ratio. Minutes, not seconds.
      '';
    };

    warnFreeGiB = mkOption {
      type = types.int;
      default = 100;
      description = "Free space below which the Waybar module turns amber.";
    };

    criticalFreeGiB = mkOption {
      type = types.int;
      default = 50;
      description = "Free space below which the Waybar module turns red.";
    };
  };

  config = mkIf cfg.enable {
    home.packages = [pkgs.heft];

    systemd.user.services.heft = {
      Unit.Description = "Disk census";
      Service = {
        Type = "oneshot";
        ExecStart = scan "--quiet";
        # a census must never compete with interactive work
        Nice = 19;
        IOSchedulingClass = "idle";
        NoNewPrivileges = true;
        RestrictSUIDSGID = true;
        LockPersonality = true;
        RestrictRealtime = true;
        SystemCallArchitectures = "native";
      };
    };

    systemd.user.timers.heft = {
      Unit.Description = "Run the disk census ${cfg.interval}";
      Timer = {
        OnCalendar = cfg.interval;
        # a laptop that was asleep at the scheduled time still gets its census
        Persistent = true;
        RandomizedDelaySec = "5m";
        Unit = "heft.service";
      };
      Install.WantedBy = ["timers.target"];
    };

    systemd.user.services.heft-deep = {
      Unit.Description = "Disk census with nix store dedup measurement";
      Service = {
        Type = "oneshot";
        ExecStart = scan "--quiet --deep";
        Nice = 19;
        IOSchedulingClass = "idle";
        NoNewPrivileges = true;
        RestrictSUIDSGID = true;
        LockPersonality = true;
        RestrictRealtime = true;
        SystemCallArchitectures = "native";
      };
    };

    systemd.user.timers.heft-deep = {
      Unit.Description = "Re-measure the nix store dedup ratio ${cfg.deepInterval}";
      Timer = {
        OnCalendar = cfg.deepInterval;
        Persistent = true;
        RandomizedDelaySec = "30m";
        Unit = "heft-deep.service";
      };
      Install.WantedBy = ["timers.target"];
    };
  };
}

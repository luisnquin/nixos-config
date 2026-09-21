{
  config,
  lib,
  pkgs,
  ...
}: let
  cfg = config.services.outage;

  seconds = description: default:
    lib.mkOption {
      type = lib.types.ints.positive;
      inherit default description;
    };

  settings = {
    inherit (cfg) user probes;
    control_group = cfg.controlGroup;
    ignore_input_devices = cfg.ignoreInputDevices;
    probe_timeout_secs = cfg.probeTimeout;
    offline_grace_secs = cfg.offlineGrace;
    idle_grace_secs = cfg.idleGrace;
    sleep_interval_secs = cfg.sleepInterval;
    network_window_secs = cfg.networkWindow;
    interaction_grace_secs = cfg.interactionGrace;
    terminate_grace_secs = cfg.terminateGrace;
    ignore_inhibitors = cfg.ignoreInhibitors;
    socket = "/run/outage/control.sock";
  };
in {
  options.services.outage = {
    enable = lib.mkEnableOption ''
      the outage protocol controller. It sits disarmed until `outage arm`;
      armed, a blackout that also takes the internet out terminates the
      desktop session and puts the machine into a sleep/wake cycle until the
      internet comes back
    '';

    package = lib.mkOption {
      type = lib.types.package;
      default = pkgs.outage;
      defaultText = lib.literalExpression "pkgs.outage";
      description = "The controller package.";
    };

    user = lib.mkOption {
      type = lib.types.str;
      description = ''
        Whose sessions the protocol terminates on entry: every login session,
        user service and background job of this user, the compositor included.
      '';
    };

    controlGroup = lib.mkOption {
      type = lib.types.str;
      default = "wheel";
      description = ''
        Group allowed to write the control socket. Members can arm and disarm
        without sudo, which is what makes a disarm possible from a console
        login while the protocol is active and the network is down.
      '';
    };

    probes = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = [
        "1.1.1.1:443"
        "8.8.8.8:443"
        "one.one.one.one:443"
      ];
      description = ''
        `host:port` targets tried in parallel; any one reachable means the
        internet is up. A hostname keeps DNS inside the check. Deliberately
        not a Tailscale or SSH probe - the protocol is about the link itself.
      '';
    };

    ignoreInputDevices = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = [];
      example = ["accelerometer"];
      description = ''
        Case-insensitive substrings of evdev device names to leave out of idle
        detection, matched against `/sys/class/input/eventN/device/name`.
        Lid and other switch events are already ignored by event type.
      '';
    };

    probeTimeout = seconds "Seconds a single connectivity probe may take." 3;
    offlineGrace = seconds "Seconds the internet must be continuously absent before entry." 120;
    idleGrace = seconds "Seconds without keyboard, pointer or touch input before entry." 300;
    sleepInterval = seconds "Seconds between scheduled wakes while the protocol is active." 600;
    networkWindow = seconds "Seconds a scheduled wake stays up for the link to come back." 60;
    interactionGrace =
      seconds ''
        Seconds the protocol stays awake after any human input, and the window an
        unscheduled wake gets. This is what stops a console login from racing
        back into suspend before a disarm can be typed.
      ''
      300;
    terminateGrace = seconds "Seconds the graceful half of the session teardown may take before the cgroup is killed." 20;

    ignoreInhibitors = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = ''
        Suspend past inhibitor locks. The user session is already gone by the
        time this matters, so a surviving inhibitor would only strand the
        protocol awake on battery.
      '';
    };
  };

  config = lib.mkIf cfg.enable {
    assertions = [
      {
        assertion = config.users.users ? ${cfg.user};
        message = "services.outage.user '${cfg.user}' is not a declared user.";
      }
      {
        assertion = config.services.greetd.enable;
        message = "services.outage needs services.greetd so the terminated session lands back on a PAM login.";
      }
    ];

    environment.systemPackages = [cfg.package];
    environment.etc."outage/config.json".text = builtins.toJSON settings;

    systemd.services.outage = {
      description = "Outage protocol controller";
      wantedBy = ["multi-user.target"];
      after = ["systemd-logind.service" "network.target"];
      wants = ["systemd-logind.service"];

      serviceConfig = {
        Type = "exec";
        ExecStart = "${lib.getExe cfg.package} daemon";
        Restart = "always";
        RestartSec = 5;
        Slice = "system.slice";
        RuntimeDirectory = "outage";
        RuntimeDirectoryMode = "0755";

        AmbientCapabilities = ["CAP_WAKE_ALARM"];

        ProtectHome = true;
        ProtectKernelModules = true;
        MemoryDenyWriteExecute = true;
        RestrictRealtime = true;

        OOMScoreAdjust = -500;
      };
    };
  };
}

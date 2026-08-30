{
  config,
  pkgs,
  lib,
  ...
}: {
  programs.waybar = {
    enable = true;
    systemd = {
      enable = false;
      targets = ["graphical-session.target"];
    };
    style = builtins.readFile ./foe.css;
    settings = let
      ewwPanel = pkgs.writeShellApplication {
        name = "eww-panel";
        runtimeInputs = [config.programs.eww.package];
        text = ''
          target="$1"
          open=$(eww active-windows 2>/dev/null || true)
          eww close-all 2>/dev/null || true
          if ! grep -q "^$target" <<<"$open"; then
            eww open "$target"
          fi
        '';
      };
      ewwToggle = name: "${lib.getExe ewwPanel} ${name}";

      ewwToggleCalendar = ewwToggle "calendar";

      ewwToggleSysmon = ewwToggle "sysmon";

      ewwToggleBattery = ewwToggle "battery";

      ewwToggleNetwork = ewwToggle "network";

      ewwToggleTailscale = ewwToggle "tailscale";

      ewwToggleHeft = ewwToggle "heft";

      # The centre marks itself read on the way out rather than on the way in,
      # so the accents that say "this is new" survive being looked at.
      notificationsToggle = pkgs.writeShellApplication {
        name = "waybar-notifications";
        runtimeInputs = [config.programs.eww.package pkgs.hark];
        text = ''
          if eww active-windows 2>/dev/null | grep -q "^notifications"; then
            eww close notifications
            hark seen
            exit 0
          fi
          eww close-all 2>/dev/null || true
          eww open notifications
        '';
      };

      # No jq here: `heft waybar` is itself a cache read that already emits the
      # bar's JSON, and degrades to a placeholder before the first scan.
      heftWaybar =
        "${lib.getExe pkgs.heft} waybar"
        + " --warn-free ${toString config.services.heft.warnFreeGiB}"
        + " --critical-free ${toString config.services.heft.criticalFreeGiB}";

      sshWaybar = pkgs.waytools.ssh;

      tailscaleWaybar = pkgs.waytools.tailscale;

      batteryWaybar = pkgs.waytools.battery;
    in [
      {
        "position" = "top";
        "layer" = "top";

        "output" = ["eDP-1"];

        modules-left = [
          "custom/launcher"
          "hyprland/workspaces"
          "tray"
        ];

        modules-center = [
          "clock"
        ];

        modules-right = [
          "custom/notifications"
          "custom/heft"
          "custom/tailscale"
          "custom/ssh"
          "group/sysmon"
          "network"
          "custom/battery"
        ];

        "group/sysmon" = {
          orientation = "vertical";
          modules = ["cpu" "memory"];
        };

        "hyprland/workspaces" = {
          "format" = "{icon}";
          "format-icons" = {
            "1" = "一";
            "2" = "二";
            "3" = "三";
            "4" = "四";
            "5" = "五";
            "6" = "六";
            "7" = "七";
            "8" = "八";
            "9" = "九";
            "10" = "十";
          };
        };

        "custom/launcher" = {
          "format" = " ";
          "tooltip" = false;
        };

        "custom/notifications" = {
          # No interval: hark streams a fresh line every time mako's D-Bus
          # property is invalidated, so the badge never lags a poll behind.
          exec = "${lib.getExe pkgs.hark} waybar --watch";
          return-type = "json";
          escape = false;
          restart-interval = 5;
          tooltip = true;
          on-click = lib.getExe notificationsToggle;
          on-click-right = "${lib.getExe pkgs.hark} dnd toggle";
          on-click-middle = "${lib.getExe pkgs.hark} clear";
        };

        "custom/ssh" = {
          exec = "${lib.getExe sshWaybar}";
          return-type = "json";
          escape = false;
          interval = 2;
          tooltip = true;
          hide-empty-text = true;
        };

        "custom/tailscale" = {
          exec = "${lib.getExe tailscaleWaybar}";
          return-type = "json";
          restart-interval = 5;
          tooltip = true;
          on-click = ewwToggleTailscale;
        };

        "custom/heft" = {
          exec = heftWaybar;
          return-type = "json";
          # the census runs on a timer; polling faster only re-reads a file
          interval = 300;
          tooltip = true;
          on-click = ewwToggleHeft;
        };

        "clock" = {
          interval = 60;
          format = " {:%H:%M}";
          tooltip = true;
          tooltip-format = "{:%A, %B %d %Y}";
          on-click = ewwToggleCalendar;
        };

        "cpu" = {
          "interval" = 1;
          "format" = "󰍛 {usage}%";
          "on-click" = ewwToggleSysmon;
        };

        "memory" = {
          "interval" = 1;
          "format" = " {percentage}%";
          "states" = {
            "warning" = 80;
            "critical" = 95;
          };
          "on-click" = ewwToggleSysmon;
        };

        "custom/battery" = {
          exec =
            "${lib.getExe batteryWaybar}"
            + " --warn ${toString config.services.battery-notifier.settings.warn.threshold}"
            + " --critical ${toString config.services.battery-notifier.settings.threat.threshold}";
          return-type = "json";
          escape = false;
          interval = 3;
          tooltip = true;
          on-click = ewwToggleBattery;
        };

        "network" = {
          "format-wifi" = "{icon}";
          "format-ethernet" = "󰈀";
          "format-linked" = "󰤫";
          "format-disconnected" = "󰤮";
          "format-icons" = ["󰤯" "󰤟" "󰤢" "󰤥" "󰤨"];
          "tooltip" = false;
          "interval" = 5;
          "on-click" = ewwToggleNetwork;
        };

        "tray" = {
          "icon-size" = 15;
          "spacing" = 10;
        };
      }
    ];
  };
}

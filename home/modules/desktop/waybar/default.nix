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
      runBtop = "${pkgs.lib.getExe config.programs.ghostty.package} --class=waybar.btop -e ${pkgs.btop}/bin/btop";

      ewwToggleCalendar = "${lib.getExe config.programs.eww.package} open --toggle calendar";

      ewwToggleNetwork = "${lib.getExe config.programs.eww.package} open --toggle network";

      sshInboundWaybar = pkgs.writeShellApplication {
        name = "ssh-inbound-waybar";
        runtimeInputs = with pkgs; [coreutils gawk jq];

        text = ''
          inbound="$(who | awk '$2 ~ /^pts\// && $NF ~ /^\(/ { n++ } END { print n+0 }')"
          inbound="''${inbound:-0}"

          if [ "$inbound" -eq 0 ]; then
            jq -cn '{text: "", tooltip: ""}'
            exit 0
          fi

          jq -cn \
            --arg text "↓ $inbound" \
            --arg tooltip "SSH inbound: $inbound" \
            '{text: $text, tooltip: $tooltip}'
        '';
      };

      sshOutboundWaybar = pkgs.writeShellApplication {
        name = "ssh-outbound-waybar";
        runtimeInputs = with pkgs; [coreutils jq procps];

        text = ''
          outbound="$( (pgrep -u "$USER" -x ssh 2>/dev/null || true) | wc -l)"
          outbound="''${outbound// /}"

          if [ "$outbound" -eq 0 ]; then
            jq -cn '{text: "", tooltip: ""}'
            exit 0
          fi

          jq -cn \
            --arg text "↑ $outbound" \
            --arg tooltip "SSH outbound: $outbound" \
            '{text: $text, tooltip: $tooltip}'
        '';
      };

      tailscaleWaybar = pkgs.writeShellApplication {
        name = "tailscale-waybar";
        runtimeInputs = with pkgs; [jq tailscale];
        text = ''
          if ! json="$(tailscale status --json 2>/dev/null)"; then
            printf '{"text":"󰖂 off","class":"disconnected"}\n'
            exit 0
          fi

          online="$(printf '%s' "$json" | jq '[.Peer[]? | select(.Online == true)] | length')"
          total="$(printf '%s' "$json" | jq '[.Peer[]?] | length')"

          # tooltip=$(tailscale status)
          tooltip="$online/$total"

          printf '{"text":"󰇢 %s/%s","tooltip":"%s"}\n' \
            "$online" "$total" "$tooltip"
        '';
      };
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
          "custom/tailscale"
          "custom/ssh-label"
          "custom/ssh-inbound"
          "custom/ssh-outbound"
          "cpu"
          "memory"
          "battery"
          "network"
        ];

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

        "custom/ssh-label" = {
          format = "󰣀";
          tooltip = "SSH sessions";
        };

        "custom/ssh-inbound" = {
          exec = "${lib.getExe sshInboundWaybar}";
          return-type = "json";
          interval = 2;
          tooltip = true;
          hide-empty-text = true;
        };

        "custom/ssh-outbound" = {
          exec = "${lib.getExe sshOutboundWaybar}";
          return-type = "json";
          interval = 2;
          tooltip = true;
          hide-empty-text = true;
        };

        "custom/tailscale" = {
          exec = "${lib.getExe tailscaleWaybar}";
          return-type = "json";
          interval = 5;
          tooltip = true;
          on-click = "${lib.getExe' pkgs.xdg-utils "xdg-open"} https://login.tailscale.com/admin/machines";
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
          "on-click" = runBtop;
        };

        "memory" = {
          "interval" = 1;
          "format" = "󰻠 {percentage}%";
          "states" = {
            "warning" = 80;
            "critical" = 95;
          };
          "on-click" = runBtop;
        };

        "battery" = {
          "states" = {
            "warning" = config.services.battery-notifier.settings.warn.threshold;
            "critical" = config.services.battery-notifier.settings.threat.threshold;
          };

          "interval" = 3;

          "format" = "{icon}";
          "format-charging" = "󰂄";
          "format-plugged" = "󱟠";
          "format-alt" = "{time} {icon}";
          "format-full" = "󱟢";

          "format-icons" = ["󰁺" "󰁻" "󰁽" "󰁿" "󰂀" "󰂂"];
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

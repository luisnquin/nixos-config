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
          "custom/network-scan"
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

        "custom/network-scan" = {
          "exec" = "${lib.getExe pkgs.scripts.nmcli-wifi-scan-waybar}";
          "return-type" = "json";
          "tooltip" = true;
          "interval" = 1;
          "tooltip-format" = "Scan Wi-Fi networks nearby";
          "on-click" = "${lib.getExe pkgs.scripts.nmcli-wifi-scan-waybar} --scan";
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
        };

        "clock" = {
          interval = 60;
          format = " {:%H:%M}";
          tooltip = true;
          tooltip-format = "<tt>{calendar}</tt>";

          calendar = {
            mode = "month";
            weeks-pos = "";
            format = {
              months = "<span color='#e8cef5'><b>{}</b></span>";
              days = "<span color='#cdd6f4'><b>{}</b></span>";
              weekdays = "<span color='#b5e8e0'><b>{}</b></span>";
              today = "<span color='#f28fad'><b><u>{}</u></b></span>";
            };
          };
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
          "format-disconnected" = "󰯡 Disconnected";
          "format-ethernet" = "↑ {bandwidthUpBytes} ↓ {bandwidthDownBytes} ({ifname})";
          "format-linked" = "󰖪 {essid} (No IP)";
          "format-wifi" = "↑ {bandwidthUpBytes} ↓ {bandwidthDownBytes} {ifname}";
          "tooltip-format" = "{ifname} ^ {essid} ({frequency}MHz)";
          "tooltip" = true;
          "interval" = 1;
          "on-click" = "${lib.getExe config.programs.ghostty.package} --class=waybar.nmtui -e ${pkgs.networkmanager}/bin/nmtui connect";
        };

        "tray" = {
          "icon-size" = 15;
          "spacing" = 10;
        };
      }
    ];
  };
}

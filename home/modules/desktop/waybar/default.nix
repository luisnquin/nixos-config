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
    style = builtins.readFile ./v1.css;
    settings = let
      runBtop = "${pkgs.lib.getExe config.programs.ghostty.package} --class=waybar.btop -e ${pkgs.btop}/bin/btop";

      sshInboundWaybar = pkgs.writeShellApplication {
        name = "ssh-inbound-waybar";
        runtimeInputs = with pkgs; [coreutils gawk jq];

        text = ''
          inbound="$(who | awk '$2 ~ /^pts\// && $NF ~ /^\(/ { n++ } END { print n+0 }')"

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
          # "user"
          "hyprland/workspaces"
          "tray"
        ];

        modules-center = [
          "custom/clock"
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
          # "custom/power"
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
          # "persistent-workspaces" = {
          #   "*" = 10;
          # };
          # "active-only" = true;
        };

        "custom/launcher" = {
          "format" = " ";
          "on-click" = "pkill rofi || rofi2";
          "on-click-middle" = "exec default_wall";
          "on-click-right" = "exec wallpaper_random";
          "tooltip" = false;
        };

        "bluetooth" = {
          # "controller": "controller1"; # specify the alias of the controller if there are more than 1 on the system
          "format" = "";
          "format-disabled" = ""; # an empty format will hide the module
          "format-connected" = " {num_connections}";
          "format-connected-battery" = " {num_connections} [{device_battery_percentage}%]";
          "tooltip-format" = "{controller_alias}\t{controller_address}";
          "tooltip-format-connected" = "{controller_alias}\t{controller_address}\n\n{device_enumerate}";
          "tooltip-format-enumerate-connected" = "{device_alias}\t{device_address}";
          "on-click" = "${lib.getExe config.programs.ghostty.package} -e ${lib.getExe pkgs.bluetuith}";
        };

        "pulseaudio" = {
          "scroll-step" = 1;
          "format" = "{icon}  {volume}%";
          "format-muted" = "󰖁 Muted";
          "format-icons" = {
            "default" = ["" "" ""];
          };
          "on-click" = "pamixer -t";
          "tooltip" = false;
        };

        "custom/mullvad" = {
          "exec" = "${lib.getExe pkgs.scripts.mullvad-status} --waybar \"{{emoji}}  {{relay-id}}\"";
          "interval" = 2;
          "return-type" = "json";
          "on-click" = "${lib.getExe pkgs.scripts.mullvad-status} --toggle-connection";
        };

        "custom/network-scan" = {
          "exec" = "${lib.getExe pkgs.scripts.nmcli-wifi-scan-waybar}";
          "return-type" = "json";
          "tooltip" = true;
          "interval" = 1;
          "tooltip-format" = "Scan wifi networks nearby";
          "on-click" = "${lib.getExe pkgs.scripts.nmcli-wifi-scan-waybar} --scan";
        };

        "custom/ssh-label" = {
          format = "󰣀";
          tooltip = false;
        };

        "custom/ssh-inbound" = {
          exec = "${lib.getExe sshInboundWaybar}";
          return-type = "json";
          interval = 2;
          tooltip = true;
        };

        "custom/ssh-outbound" = {
          exec = "${lib.getExe sshOutboundWaybar}";
          return-type = "json";
          interval = 2;
          tooltip = true;
        };

        "custom/tailscale" = {
          exec = "${lib.getExe tailscaleWaybar}";
          return-type = "json";
          interval = 5;
          tooltip = true;
        };

        "custom/clock" = {
          "exec" = "date +' %H:%M'";
          "interval" = 2;
          "tooltip" = false;
          "on-click" = "${pkgs.gsimplecal}/bin/gsimplecal";
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
          "format" = "{icon} {capacity}%";
          "format-charging" = "󰂄 {capacity}%";
          "format-plugged" = "󱟠 {capacity}%";
          "format-alt" = "{time} {icon}";
          "format-full" = "󱟢 {capacity}%";
          "format-icons" = ["󰁺" "󰁻" "󰁽" "󰁿" "󰂀" "󰂂"];
        };

        "network" = {
          "format-disconnected" = "󰯡 Disconnected";
          "format-ethernet" = "↑ {bandwidthUpBytes} ↓ {bandwidthDownBytes} 󰀂  󰖩  {ifname} ({ipaddr})";
          "format-linked" = "󰖪 {essid} (No IP)";
          "format-wifi" = "↑ {bandwidthUpBytes} ↓ {bandwidthDownBytes}";
          "tooltip-format" = "{ifname} ^ {essid} ({frequency}MHz)"; # 󰤨
          "tooltip" = true;
          "interval" = 1;
          "on-click" = "${lib.getExe config.programs.ghostty.package} --class=waybar.nmtui -e ${pkgs.networkmanager}/bin/nmtui connect";
        };

        "custom/power" = {
          "format" = "";
          "on-click" = ''${pkgs.rofi}/bin/rofi -modi "power-menu:${pkgs.rofi-power-menu}/bin/rofi-power-menu" -show power-menu'';
          "tooltip" = false;
        };

        "tray" = {
          "icon-size" = 15;
          "spacing" = 10;
        };

        "user" = {
          "format" = "{avatar}";
          "interval" = 60;
          "height" = 30;
          "width" = 30;
          "icon" = true;
        };
      }
    ];
  };
}

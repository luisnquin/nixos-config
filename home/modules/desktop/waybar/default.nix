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
      ewwToggleCalendar = "${lib.getExe config.programs.eww.package} open --toggle calendar";

      ewwToggleCpu = "${lib.getExe config.programs.eww.package} open --toggle cpu";

      ewwToggleMemory = "${lib.getExe config.programs.eww.package} open --toggle memory";

      ewwToggleBattery = "${lib.getExe config.programs.eww.package} open --toggle battery";

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

      batteryWaybar = pkgs.writeShellApplication {
        name = "battery-waybar";
        runtimeInputs = with pkgs; [coreutils gawk jq];
        text = ''
          bat=""
          for path in /sys/class/power_supply/*; do
            [ -r "$path/type" ] || continue
            [ "$(cat "$path/type")" = "Battery" ] || continue
            bat="$path"
            break
          done

          if [ -z "$bat" ]; then
            jq -cn '{text:"?", tooltip:"No battery", class:"missing"}'
            exit 0
          fi

          status="$(cat "$bat/status" 2>/dev/null || echo Unknown)"
          capacity="$(cat "$bat/capacity" 2>/dev/null || echo 0)"

          ac_online=0
          for path in /sys/class/power_supply/*; do
            [ -r "$path/type" ] || continue
            type="$(cat "$path/type")"
            if [ "$type" = "Mains" ] || [ "$type" = "USB" ]; then
              online="$(cat "$path/online" 2>/dev/null || echo 0)"
              [ "$online" = "1" ] && ac_online=1
            fi
          done

          read -r icon class <<EOF
          $(awk -v status="$status" -v capacity="$capacity" -v ac_online="$ac_online" \
            -v warn="${toString config.services.battery-notifier.settings.warn.threshold}" \
            -v critical="${toString config.services.battery-notifier.settings.threat.threshold}" '
            BEGIN {
              class = "normal";
              if (capacity <= critical) class = "critical";
              else if (capacity <= warn) class = "warning";

              if (status == "Charging") icon = "󰂄";
              else if (status == "Full") icon = "󰂂";
              else if (ac_online == 1) icon = "󰂄";
              else if (capacity < 15) icon = "󰁺";
              else if (capacity < 30) icon = "󰁻";
              else if (capacity < 45) icon = "󰁽";
              else if (capacity < 60) icon = "󰁿";
              else if (capacity < 80) icon = "󰂀";
              else icon = "󰂂";

              print icon, class;
            }')
          EOF

          jq -cn \
            --arg text "$icon" \
            --arg tooltip "$capacity% · $status" \
            --arg class "$class" \
            '{text:$text, tooltip:$tooltip, class:$class}'
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
          "network"
          "custom/ssh-label"
          "custom/ssh-inbound"
          "custom/ssh-outbound"
          "cpu"
          "memory"
          "custom/battery"
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
          "on-click" = ewwToggleCpu;
        };

        "memory" = {
          "interval" = 1;
          "format" = "󰻠 {percentage}%";
          "states" = {
            "warning" = 80;
            "critical" = 95;
          };
          "on-click" = ewwToggleMemory;
        };

        "custom/battery" = {
          exec = "${lib.getExe batteryWaybar}";
          return-type = "json";
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

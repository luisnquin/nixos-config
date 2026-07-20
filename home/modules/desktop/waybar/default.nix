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

      ewwToggleSysmon = "${lib.getExe config.programs.eww.package} open --toggle sysmon";

      ewwToggleBattery = "${lib.getExe config.programs.eww.package} open --toggle battery";

      ewwToggleNetwork = "${lib.getExe config.programs.eww.package} open --toggle network";

      ewwToggleGithub = "${lib.getExe config.programs.eww.package} open --toggle github";

      githubWaybar = pkgs.writeShellApplication {
        name = "github-monitor-waybar";
        runtimeInputs = with pkgs; [jq];
        text = ''
          cache=${lib.escapeShellArg "${config.xdg.cacheHome}/github-monitor/state.json"}
          if [ ! -r "$cache" ]; then
            jq -cn '{text:" …", tooltip:"GitHub monitor is waiting for its first refresh", class:"loading"}'
            exit 0
          fi

          jq -c '
            (.failed_count + .issue_count) as $total |
            {text:(" " + ($total | tostring)),
             tooltip:((.failed_count | tostring) + " failed of " + (.workflow_count | tostring) + " watched workflows · " +
                      (.issue_count | tostring) + " relevant open issues" +
                      (if (.errors | length) > 0 then " · " + ((.errors | length) | tostring) + " query errors" else "" end)),
             class:(if (.errors | length) > 0 then "error" elif .failed_count > 0 then "failed" elif .issue_count > 0 then "issues" else "clear" end)}' \
            "$cache"
        '';
      };

      sshWaybar = pkgs.writeShellApplication {
        name = "ssh-waybar";
        runtimeInputs = with pkgs; [coreutils gawk jq procps];

        text = ''
          inbound="$(who | awk '$2 ~ /^pts\// && $NF ~ /^\(/ { n++ } END { print n+0 }')"
          inbound="''${inbound:-0}"

          outbound="$( (pgrep -u "$USER" -x ssh 2>/dev/null || true) | wc -l)"
          outbound="''${outbound// /}"

          if [ "$inbound" -eq 0 ] && [ "$outbound" -eq 0 ]; then
            jq -cn '{text: "", tooltip: ""}'
            exit 0
          fi

          text='<span color="#cba6f7" size="15pt">󰣀</span>'
          tooltip=""

          if [ "$inbound" -gt 0 ]; then
            printf -v text '%s <span color="#b5e8e0">↓%s</span>' "$text" "$inbound"
            tooltip="SSH inbound: $inbound"
          fi

          if [ "$outbound" -gt 0 ]; then
            printf -v text '%s <span color="#d8b4fe">↑%s</span>' "$text" "$outbound"
            tooltip="''${tooltip:+$tooltip · }SSH outbound: $outbound"
          fi

          jq -cn \
            --arg text "$text" \
            --arg tooltip "$tooltip" \
            '{text: $text, tooltip: $tooltip}'
        '';
      };

      tailscaleWaybar = pkgs.writeShellApplication {
        name = "tailscale-waybar";
        runtimeInputs = with pkgs; [jq tailscale];
        text = ''
          icon=$'\ue9ff'

          if ! json="$(tailscale status --json 2>/dev/null)"; then
            printf '{"text":"%s off","class":"disconnected"}\n' "$icon"
            exit 0
          fi

          online="$(printf '%s' "$json" | jq '[.Peer[]? | select(.Online == true)] | length')"
          total="$(printf '%s' "$json" | jq '[.Peer[]?] | length')"

          # tooltip=$(tailscale status)
          tooltip="$online/$total"

          printf '{"text":"%s %s/%s","tooltip":"%s"}\n' \
            "$icon" "$online" "$total" "$tooltip"
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

          class="$(awk -v capacity="$capacity" \
            -v warn="${toString config.services.battery-notifier.settings.warn.threshold}" \
            -v critical="${toString config.services.battery-notifier.settings.threat.threshold}" '
            BEGIN {
              if (capacity <= critical) print "critical";
              else if (capacity <= warn) print "warning";
              else print "normal";
            }')"

          # horizontal font-awesome battery glyphs (nf-fa-battery_*)
          if [ "$capacity" -lt 12 ]; then icon=$'\uf244'
          elif [ "$capacity" -lt 37 ]; then icon=$'\uf243'
          elif [ "$capacity" -lt 62 ]; then icon=$'\uf242'
          elif [ "$capacity" -lt 87 ]; then icon=$'\uf241'
          else icon=$'\uf240'
          fi

          if [ "$status" = "Charging" ] || { [ "$status" != "Full" ] && [ "$ac_online" = "1" ]; }; then
            icon="$icon"$' \uf0e7'
          fi

          jq -cn \
            --arg text "<span size=\"7.5pt\">$capacity%</span> <span size=\"10pt\">$icon</span>" \
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
          "custom/github-monitor"
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
          interval = 5;
          tooltip = true;
          on-click = "${lib.getExe' pkgs.xdg-utils "xdg-open"} https://login.tailscale.com/admin/machines";
        };

        "custom/github-monitor" = {
          exec = "${lib.getExe githubWaybar}";
          return-type = "json";
          interval = 30;
          tooltip = true;
          on-click = ewwToggleGithub;
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
          exec = "${lib.getExe batteryWaybar}";
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

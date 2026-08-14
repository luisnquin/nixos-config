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

      # No jq here: `heft waybar` is itself a cache read that already emits the
      # bar's JSON, and degrades to a placeholder before the first scan.
      heftWaybar = "${lib.getExe pkgs.heft} waybar"
        + " --warn-free ${toString config.services.heft.warnFreeGiB}"
        + " --critical-free ${toString config.services.heft.criticalFreeGiB}";

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

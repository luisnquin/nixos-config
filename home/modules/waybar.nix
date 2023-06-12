{pkgs, ...}: {
  # https://github.com/georgewhewell/nixos-host/blob/master/home/waybar.nix
  programs.waybar = {
    enable = true;
    systemd.enable = true;
    settings = [
      {
        height = 30;
        layer = "top";
        position = "bottom";
        tray = {spacing = 10;};
        modules-center = ["sway/window"];
        modules-left = ["sway/workspaces" "sway/mode"];
        modules-right = [
          "pulseaudio"
          "network"
          "cpu"
          "memory"
          "temperature"
          "clock"
          "tray"
        ];

        battery = {
          interval = 30;
          states = {
            critical = 20;
            warning = 35;
          };

          format = "{capacity}% {icon}";
          format-alt = "{time} {icon}";
          format-charging = "{capacity}% ";
          format-icons = ["" "" "" "" ""];
          format-plugged = "{capacity}% ";
        };

        clock = {
          format-alt = "{:%Y-%m-%d}";
          tooltip-format = "{:%Y-%m-%d | %H:%M}";
        };

        cpu = {
          interval = 8;
          format = "{usage}% ";
          tooltip = false;
        };

        memory = {
          format = "{}% ";
        };

        network = {
          interval = 1;
          format-alt = "{ifname}: {ipaddr}/{cidr}";
          format-disconnected = "Disconnected ⚠";
          format-ethernet = "{ifname}: {ipaddr}/{cidr}   up: {bandwidthUpBits} down: {bandwidthDownBits}";
          format-linked = "{ifname} (No IP) ";
          format-wifi = "{essid} ({signalStrength}%) ";
        };

        pulseaudio = {
          format = "{volume}% {icon} {format_source}";

          format-muted = " {format_source}";
          format-source = "{volume}% ";
          format-source-muted = "";

          format-bluetooth = "{volume}% {icon} {format_source}";
          format-bluetooth-muted = " {icon} {format_source}";

          format-icons = {
            car = "";
            default = ["" "" ""];
            handsfree = "";
            headphones = "";
            headset = "";
            phone = "";
            portable = "";
          };

          on-click = "pavucontrol";
        };

        "sway/mode" = {
          format = ''<span style="italic">{}</span>'';
        };

        temperature = {
          critical-threshold = 80;
          format = "{temperatureC}°C {icon}";
          format-icons = ["" "" ""];
        };
      }
    ];
  };
}

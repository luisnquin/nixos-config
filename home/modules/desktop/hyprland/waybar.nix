{
  rofi-network-manager,
  mullx,
  pkgs,
  ...
}: {
  programs.waybar = {
    enable = true;
    systemd = {
      enable = false;
      target = "graphical-session.target";
    };
    style = builtins.readFile ./styles/waybar-v1.css;
    settings = let
      runBtopWithAlacritty = "${pkgs.alacritty}/bin/alacritty -e ${pkgs.btop}/bin/btop";
      mullxBin = "${mullx}/bin/mullx";
    in [
      {
        "position" = "top";
        "layer" = "top";

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
          # "pulseaudio"
          # "backlight"
          "custom/mullvad"
          "bluetooth"
          "memory"
          "cpu"
          "battery"
          "network"
          "custom/power"
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
          "on-click" = "${pkgs.blueman}/bin/blueman-manager";
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
          "exec" = "${mullxBin} --waybar \"{{emoji}}  {{network-ip}}\"";
          "interval" = 2;
          "return-type" = "json";
          "on-click" = "${mullxBin} --toggle-connection";
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
          "on-click" = runBtopWithAlacritty;
        };

        "memory" = {
          "interval" = 1;
          "format" = "󰻠 {percentage}%";
          "states" = {
            "warning" = 80;
            "critical" = 95;
          };
          "on-click" = runBtopWithAlacritty;
        };

        "battery" = {
          "states" = {
            "warning" = 30;
            "critical" = 15;
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
          "format-ethernet" = "󰒢 Connected!";
          "format-linked" = "󰖪  {essid} (No IP)";
          "format-wifi" = "󰖩  {ipaddr}";
          "interval" = 5;
          "tooltip" = true;
          "tooltip-format" = "{ifname} ^ {essid} ({frequency}MHz)"; # 󰤨
          "on-click" = "${rofi-network-manager}/bin/rofi-network-manager";
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

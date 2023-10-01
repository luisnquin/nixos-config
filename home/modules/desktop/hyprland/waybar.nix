{
  rofi-network-manager,
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
      mullvanBin = "${pkgs.callPackage ./../../../scripts/mullman {}}/bin/mullman";
    in [
      {
        "position" = "top";
        "layer" = "top";

        modules-left = [
          "custom/launcher"
          "user"
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
          "persistent-workspaces" = {
            "*" = 10;
          };
        };

        "custom/launcher" = {
          "format" = " ";
          "on-click" = "pkill rofi || rofi2";
          "on-click-middle" = "exec default_wall";
          "on-click-right" = "exec wallpaper_random";
          "tooltip" = false;
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
          "exec" = "${mullvanBin} --waybar \"{{emoji}}  {{network-ip}}\"";
          "interval" = 2;
          "return-type" = "json";
          "on-click" = "${mullvanBin} --toggle-connection";
        };

        "custom/clock" = {
          "exec" = "date +'%Y-%m-%d %H:%M'";
          "interval" = 2;
          "tooltip" = false;
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
            "warning" = 85;
          };
          "on-click" = runBtopWithAlacritty;
        };

        "battery" = {
          "states" = {
            "warning" = 30;
            "critical" = 15;
          };
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
          "format-wifi" = "󰖩  {essid}";
          "interval" = 1;
          "tooltip" = false;
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

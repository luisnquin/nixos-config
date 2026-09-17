{
  config,
  lib,
  pkgs,
  widgets,
  ...
}: {
  programs.waybar = {
    enable = true;
    systemd = {
      enable = false;
      targets = ["graphical-session.target"];
    };
    style = builtins.readFile ./waybar.css;
    settings = [
      ({
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
            "group/ssh"
            "group/sysmon"
            "network"
            "custom/battery"
          ];

          # The monogram sits beside the sessions, not inside each row: repeating
          # it per row is what broke the alignment against group/sysmon. Exactly
          # one of ssh-solo and ssh-sessions carries text at any time.
          "group/ssh" = {
            orientation = "horizontal";
            modules = ["custom/ssh-icon" "custom/ssh-solo" "group/ssh-sessions"];
          };

          "group/ssh-sessions" = {
            orientation = "vertical";
            modules = ["custom/ssh-in" "custom/ssh-out"];
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

          "custom/ssh-icon" = {
            format = "󰣀";
            tooltip = false;
          };

          "custom/ssh-solo" = {
            exec = "${lib.getExe pkgs.barfeed.sshSolo}";
            return-type = "json";
            escape = false;
            interval = 2;
            tooltip = true;
            hide-empty-text = true;
          };

          "custom/ssh-in" = {
            exec = "${lib.getExe pkgs.barfeed.sshIn}";
            return-type = "json";
            escape = false;
            interval = 2;
            tooltip = true;
            hide-empty-text = true;
          };

          "custom/ssh-out" = {
            exec = "${lib.getExe pkgs.barfeed.sshOut}";
            return-type = "json";
            escape = false;
            interval = 2;
            tooltip = true;
            hide-empty-text = true;
          };

          "tray" = {
            "icon-size" = 15;
            "spacing" = 10;
          };
        }
        // lib.mergeAttrsList (map (w: w.bar) widgets))
    ];
  };
}

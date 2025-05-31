args @ {
  nixosConfig,
  pkgs,
  lib,
  ...
}: let
  mainMod = "SUPER";

  execOnce = [
    "${pkgs.wl-clipboard}/bin/wl-paste --watch ${pkgs.cliphist}/bin/cliphist store -max-items 200"
    "dbus-update-activation-environment --systemd WAYLAND_DISPLAY XDG_CURRENT_DESKTOP"
    "[workspace 2 silent] ghostty"
    "${pkgs.extra.hyprdrop}/bin/hyprdrop -i ghostty.hyprdrop 'ghostty --class=ghostty.hyprdrop'; while [ -z \"$ADDRESS\" ] ; do ADDRESS=$(hyprctl clients -j | jq -r '.[] | select(.class == \"ghostty.hyprdrop\") | .address'); done; ${pkgs.extra.hyprdrop}/bin/hyprdrop -i ghostty.hyprdrop 'ghostty --class=ghostty.hyprdrop'"
  ];

  execAlways = [
    "pkill waybar & sleep 0.1 && waybar"
  ];
in {
  imports = [
    ../components/swww.nix
    ../components/mako.nix
    ../components/waybar

    ../../cursor.nix
    ../../rofi.nix
    ../../gtk.nix
  ];

  wayland.windowManager.hyprland = {
    enable = true;
    settings = {
      monitor = ",preferred,auto,1,mirror,eDP-1";
      env = "XCURSOR_SIZE,24";

      general = {
        layout = "dwindle";
        gaps_in = 4;
        gaps_out = 20;
        border_size = 1;

        "col.active_border" = "rgba(595959aa)";
        "col.inactive_border" = "rgba(595959aa)";

        windowrule = [
          "opacity 1.0 override 1.0 override,class:Emulator"
        ];

        windowrulev2 = [
          "float,center,forceinput, class:(Rofi)$"
          "float, title:^(Android Emulator -)"
          "float, title:^(Emulator)$"
          "float,center, class:^(ghostty.hyprdrop)$"
          "size 1280 720, class:^(ghostty.hyprdrop)$"
          "float,center, class:^(waybar.nmtui)$"
          "size 1050 720, class:^(waybar.nmtui)$"
        ];
      };

      decoration = {
        rounding = 4;
        blur = {
          enabled = true;
          size = 3;
          passes = 1;
        };
        active_opacity = 0.9;
        inactive_opacity = 1.0;
        fullscreen_opacity = 1.0;
        shadow = {
          enabled = true;
          range = 4;
          render_power = 3;
          color = "rgba(1a1a1aee)";
        };
      };

      animations = {
        enabled = true;
        bezier = [
          "myBezier, 0.05, 0.9, 0.1, 1.05"
        ];
        animation = [
          "windows, 1, 7, myBezier"
          "windowsOut, 1, 7, default, popin 80%"
          "border, 1, 10, default"
          "borderangle, 1, 8, default"
          "fade, 1, 7, default"
          "workspaces, 1, 6, default"
        ];
      };

      input = {
        numlock_by_default = true;
        sensitivity = 0.2;

        kb_layout = "latam";
        kb_variant = "";
        kb_model = "";
        kb_options = "";
        kb_rules = "";

        follow_mouse = 1;

        touchpad = {
          natural_scroll = "no";
          disable_while_typing = "no";
        };
      };

      misc = {
        disable_autoreload = true;
        disable_hyprland_logo = true;
        focus_on_activate = false;
        new_window_takes_over_fullscreen = 1;
        initial_workspace_tracking = 2;
      };

      dwindle = {
        # See https://wiki.hyprland.org/Configuring/Dwindle-Layout/ for more
        pseudotile = true;
        preserve_split = true;
      };

      # See https://wiki.hyprland.org/Configuring/Master-Layout/ for more
      master = {
        new_status = "master";
      };

      gestures = {
        workspace_swipe = "off";
      };

      ecosystem = {
        no_update_news = true;
        enforce_permissions = true;
      };

      device = {
        name = "epic-mouse-v1";
        sensitivity = "-0.5";
      };

      bind = import ./binds.nix args;
      bindm = [
        "${mainMod}, mouse:272, movewindow"
        "${mainMod}, mouse:273, resizewindow"
      ];

      permission = [
        "${lib.getExe pkgs.hyprpicker}, screencopy, allow"
        "${lib.getExe pkgs.grim}, screencopy, allow"
        "/nix/store/*-zen*/bin/(zen|zen-beta|zen-twilight), screencopy, allow"
        "${lib.getExe pkgs.brave}, screencopy, allow"
        "${lib.getExe pkgs.discord-canary}, screencopy, allow"
        "${lib.getExe pkgs.obs-studio}, screencopy, allow"
        "${nixosConfig.programs.hyprland.portalPackage}/bin/xdg-desktop-portal-hyprland, screencopy, allow"
      ];
    };
    extraConfig = ''
      $mainMod = ${mainMod}

      ${lib.concatMapStrings (cmd: "exec-once = ${cmd}\n") execOnce}
      ${lib.concatMapStrings (cmd: "exec = ${cmd}\n") execAlways}
    '';
  };
}

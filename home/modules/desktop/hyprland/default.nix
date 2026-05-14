# Hyprland 0.55+ reads hyprland.lua; bump wayland.windowManager.hyprland.package if yours is older.
args @ {
  pkgs,
  lib,
  config,
  ...
}: let
  inherit (lib.generators) mkLuaInline;
  hyprctlCmd = "${pkgs.hyprland}/bin/hyprctl";

  waybarRestart = pkgs.writeShellScript "hypr-waybar-restart" ''
    pkill waybar 2>/dev/null || true
    sleep 0.1
    exec ${lib.getExe config.programs.waybar.package}
  '';

  startupBody = ''
    hl.exec_cmd("${pkgs.wl-clipboard}/bin/wl-paste --watch ${pkgs.cliphist}/bin/cliphist store -max-items 200")
    hl.exec_cmd("dbus-update-activation-environment --systemd WAYLAND_DISPLAY XDG_CURRENT_DESKTOP")
    hl.exec_cmd("${waybarRestart}")
    hl.exec_cmd("[workspace 2 silent] ${lib.getExe config.programs.ghostty.package}")
    hl.exec_cmd("${pkgs.writeShellScript "hypr-ghostty-drop" ''
      ${lib.getExe pkgs.hyprdrop} -i ghostty.hyprdrop "ghostty --class=ghostty.hyprdrop"
      while [ -z "$ADDRESS" ]; do
        ADDRESS=$(${hyprctlCmd} clients -j | ${lib.getExe pkgs.jq} -r '.[] | select(.class == "ghostty.hyprdrop") | .address')
      done
      exec ${lib.getExe pkgs.hyprdrop} -i ghostty.hyprdrop "ghostty --class=ghostty.hyprdrop"
    ''}")
  '';

  waybarReload = ''hl.exec_cmd("${waybarRestart}")'';
in {
  home.sessionVariables = {
    GRIMBLAST_HIDE_CURSOR = 0;
  };

  wayland.windowManager.hyprland = {
    enable = true;
    configType = "lua";

    settings = {
      monitor = [
        {
          output = "eDP-1";
          mode = "1920x1080@144";
          position = "0x0";
          scale = 1;
        }
        {
          output = "HDMI-A-1";
          mode = "1920x1080@60";
          position = "1920x0";
          scale = 1;
        }
        {
          output = "DP-1";
          disabled = true;
        }
      ];

      env = [
        {
          _args = ["XCURSOR_SIZE" "24"];
        }
        {
          _args = ["HYPRCURSOR_SIZE" "24"];
        }
      ];

      config = {
        general = {
          layout = "dwindle";
          gaps_in = 4;
          gaps_out = 20;
          border_size = 1;
          col = {
            active_border = "rgba(595959aa)";
            inactive_border = "rgba(595959aa)";
          };
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

        debug = {
          disable_logs = false;
        };

        animations = {
          enabled = true;
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
            natural_scroll = false;
            disable_while_typing = false;
          };
        };

        misc = {
          disable_autoreload = true;
          disable_hyprland_logo = true;
          focus_on_activate = false;
          initial_workspace_tracking = 0;
        };

        dwindle = {
          preserve_split = true;
        };

        master = {
          new_status = "master";
        };

        ecosystem = {
          no_update_news = true;
          enforce_permissions = true;
        };
      };

      curve = [
        {
          _args = [
            "myBezier"
            {
              type = "bezier";
              points = [
                [0.05 0.9]
                [0.1 1.05]
              ];
            }
          ];
        }
      ];

      animation = [
        {
          leaf = "windows";
          enabled = true;
          speed = 7;
          bezier = "myBezier";
        }
        {
          leaf = "windowsOut";
          enabled = true;
          speed = 7;
          bezier = "default";
          style = "popin 80%";
        }
        {
          leaf = "border";
          enabled = true;
          speed = 10;
          bezier = "default";
        }
        {
          leaf = "borderangle";
          enabled = true;
          speed = 8;
          bezier = "default";
        }
        {
          leaf = "fade";
          enabled = true;
          speed = 7;
          bezier = "default";
        }
        {
          leaf = "workspaces";
          enabled = true;
          speed = 6;
          bezier = "default";
        }
      ];

      window_rule = [
        {
          name = "emulator-opacity";
          match = {class = "^Emulator$";};
          opacity = "1.0 override 1.0 override 1.0 override";
        }
        {
          name = "android-emulator-float";
          match = {title = "^(Android Emulator -)";};
          float = true;
        }
        {
          name = "emulator-title-float";
          match = {title = "^(Emulator)$";};
          float = true;
        }
        {
          name = "ghostty-hyprdrop";
          match = {class = "^ghostty\\.hyprdrop$";};
          float = true;
          size = "1280 720";
          center = true;
        }
        {
          name = "waybar-nmtui";
          match = {class = "^waybar\\.nmtui$";};
          float = true;
          size = "1050 720";
          center = true;
        }
        {
          name = "waybar-btop";
          match = {class = "^waybar\\.btop$";};
          float = true;
          size = "1280 720";
          center = true;
        }
        {
          name = "pip";
          match = {title = "^(Picture in picture|Picture-in-Picture)";};
          size = "640 360";
          float = true;
          pin = true;
        }
        {
          name = "raffi-float";
          match = {class = "^com\\.chmouel\\.raffi$";};
          float = true;
          center = true;
        }
      ];

      device = {
        name = "epic-mouse-v1";
        sensitivity = -0.5;
      };

      bind = import ./binds.nix args;

      permission = [
        {
          _args = ["${lib.getExe pkgs.hyprpicker}" "screencopy" "allow"];
        }
        {
          _args = ["${lib.getExe pkgs.grim}" "screencopy" "allow"];
        }
        {
          _args = ["/nix/store/[^/]+-zen[^/]*/bin/(zen|zen-beta|zen-twilight)" "screencopy" "allow"];
        }
        {
          _args = ["${lib.getExe pkgs.brave}" "screencopy" "allow"];
        }
        {
          _args = ["${lib.getExe pkgs.obs-studio}" "screencopy" "allow"];
        }
        {
          _args = ["${lib.getExe pkgs.xdg-desktop-portal-hyprland}" "screencopy" "allow"];
        }
        {
          _args = ["${lib.getExe config.programs.nixcord.finalPackage.discord}" "screencopy" "allow"];
        }
      ];

      on = [
        {
          _args = [
            "hyprland.start"
            (mkLuaInline ''
              function()
                ${startupBody}
              end
            '')
          ];
        }
        {
          _args = [
            "config.reloaded"
            (mkLuaInline ''
              function()
                ${waybarReload}
              end
            '')
          ];
        }
      ];
    };

    extraConfig = "";
  };
}

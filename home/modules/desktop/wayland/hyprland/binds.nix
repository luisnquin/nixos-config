# https://wiki.hyprland.org/Configuring/Keywords/
{
  config,
  inputs,
  system,
  pkgs,
  ...
}: let
  mainMod = "SUPER";

  window = [
    "${mainMod}, left, movefocus, l"
    "${mainMod}, right, movefocus, r"
    "${mainMod}, up, movefocus, u"
    "${mainMod}, down, movefocus, d"
  ];

  workspaces = [
    # Go-to
    "${mainMod}, 1, workspace, 1"
    "${mainMod}, 2, workspace, 2"
    "${mainMod}, 3, workspace, 3"
    "${mainMod}, 4, workspace, 4"
    "${mainMod}, 5, workspace, 5"
    "${mainMod}, 6, workspace, 6"
    "${mainMod}, 7, workspace, 7"
    "${mainMod}, 8, workspace, 8"
    "${mainMod}, 9, workspace, 9"
    "${mainMod}, 0, workspace, 10"

    # Move window
    "${mainMod} SHIFT, 1, movetoworkspace, 1"
    "${mainMod} SHIFT, 2, movetoworkspace, 2"
    "${mainMod} SHIFT, 3, movetoworkspace, 3"
    "${mainMod} SHIFT, 4, movetoworkspace, 4"
    "${mainMod} SHIFT, 5, movetoworkspace, 5"
    "${mainMod} SHIFT, 6, movetoworkspace, 6"
    "${mainMod} SHIFT, 7, movetoworkspace, 7"
    "${mainMod} SHIFT, 8, movetoworkspace, 8"
    "${mainMod} SHIFT, 9, movetoworkspace, 9"
    "${mainMod} SHIFT, 0, movetoworkspace, 10"

    # Resize window
    "${mainMod} SHIFT, h, resizeactive, -40 0"
    "${mainMod} SHIFT, l, resizeactive, 40 0"
    "${mainMod} SHIFT, k, resizeactive, 0 -40"
    "${mainMod} SHIFT, j, resizeactive, 0 40"

    # Scroll
    "${mainMod}, mouse_down, workspace, e+1"
    "${mainMod}, mouse_up, workspace, e-1"

    # Other actions
    "${mainMod}, SPACE, togglefloating"
    "${mainMod}, MINUS, togglesplit"
    "SUPER_SHIFT, W, killactive"
    "SUPER_SHIFT, MINUS, pseudo"
    "${mainMod}, F, fullscreen"
  ];

  custom = let
    rofi-plugin-call = name: program-to-exec: ''${lib.getExe pkgs.rofi} -modi "${name}:${program-to-exec}" -show ${name}'';
    spotify-dbus-control = inputs.spotify-dbus-control.defaultPackage.${system};
    hyprstfu = inputs.hyprstfu.defaultPackage.${system};

    inherit (inputs.nix-scripts.packages.${system}) sys-sound sys-brightness cliphist-rofi;

    grimblast = inputs.hyprland-contrib.packages.${system}.grimblast.overrideAttrs (_oldAttrs: {
      prePatch = ''
        substituteInPlace ./grimblast --replace '-t 3000' '-t 3000 -i ${../../../../dots/icons/crop.512.png}'
      '';
    });

    inherit (pkgs) lib;
  in [
    {
      "mod+key" = "${mainMod}, RETURN";
      "dispatcher" = "exec, ${pkgs.lib.getExe config.programs.ghostty.package}";
    }
    {
      "mod+key" = ",XF86AudioMicMute";
      "dispatcher" = "exec, ${lib.getExe sys-sound} --toggle-mic";
    }
    {
      "mod+key" = ",XF86AudioMute";
      "dispatcher" = "exec, ${lib.getExe sys-sound} --toggle-vol";
    }
    {
      "mod+key" = ",XF86AudioLowerVolume";
      "dispatcher" = "exec, ${lib.getExe sys-sound} --dec";
    }
    {
      "mod+key" = ",XF86AudioRaiseVolume";
      "dispatcher" = "exec, ${lib.getExe sys-sound} --inc";
    }
    {
      "mod+key" = "SUPER, XF86AudioRaiseVolume";
      "dispatcher" = "exec, ${lib.getExe sys-sound} --inc --unleashed";
    }
    {
      "mod+key" = "SUPER_SHIFT, Print";
      "dispatcher" = "exec, ${lib.getExe grimblast} --freeze --notify copy area";
    }
    {
      "mod+key" = "${mainMod}, Print";
      "dispatcher" = "exec, ${lib.getExe grimblast} --notify copy active";
    }
    {
      "mod+key" = ",Print";
      "dispatcher" = "exec, ${lib.getExe grimblast} --notify copy screen";
    }
    {
      "mod+key" = ",XF86MonBrightnessDown";
      "dispatcher" = "exec, ${lib.getExe sys-brightness} --dec";
    }
    {
      "mod+key" = ",XF86MonBrightnessUp";
      "dispatcher" = "exec, ${lib.getExe sys-brightness} --inc";
    }
    {
      "mod+key" = "SUPER_SHIFT, C";
      "dispatcher" = "exec, ${rofi-plugin-call "clipboard" "${lib.getExe cliphist-rofi}"}";
    }
    {
      "mod+key" = "SUPER_SHIFT, Q";
      "dispatcher" = "exec, ${lib.getExe pkgs.rofi} -show window";
    }
    {
      "mod+key" = "${mainMod}, Q";
      "dispatcher" = "exec, ${lib.getExe pkgs.rofi} -show drun";
    }
    {
      "mod+key" = "SUPER_SHIFT, E";
      "dispatcher" = "exec, ${lib.getExe pkgs.bemoji}";
    }
    {
      "mod+key" = "CTRL_SHIFT, braceleft";
      "dispatcher" = "exec, ${lib.getExe spotify-dbus-control} --push-back";
    }
    {
      "mod+key" = "CTRL_SHIFT, braceright";
      "dispatcher" = "exec, ${lib.getExe spotify-dbus-control} --push-forward";
    }
    {
      "mod+key" = "SUPER_SHIFT, braceright";
      "dispatcher" = "exec, ${lib.getExe spotify-dbus-control} --next";
    }
    {
      "mod+key" = "SUPER_SHIFT, braceleft";
      "dispatcher" = "exec, ${lib.getExe spotify-dbus-control} --prev";
    }
    {
      "mod+key" = "${mainMod}, Pause";
      "dispatcher" = "exec, ${lib.getExe spotify-dbus-control} --toggle";
    }
    {
      "mod+key" = "SUPER_SHIFT, R";
      "dispatcher" = "exec, ${pkgs.hyprland}/bin/hyprctl reload";
    }
    {
      "mod+key" = "SUPER_SHIFT, S";
      "dispatcher" = "exec, ${pkgs.toybox}/bin/pkill glava || ${lib.getExe pkgs.glava} -d";
    }
    {
      "mod+key" = "${mainMod}, M";
      "dispatcher" = "exec, ${lib.getExe hyprstfu}";
    }
    {
      "mod+key" = "${mainMod}, K";
      "dispatcher" = "exec, ${lib.getExe pkgs.extra.hyprdrop} -i ghostty.hyprdrop 'ghostty --class=ghostty.hyprdrop'";
    }
  ];
in
  window ++ workspaces ++ builtins.map (b: b."mod+key" + ", " + b.dispatcher) custom

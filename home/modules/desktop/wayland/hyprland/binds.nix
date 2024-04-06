# https://wiki.hyprland.org/Configuring/Keywords/
{
  sys-brightness,
  sys-sound,
  cliphist-rofi,
  spotify-dbus,
  grimblast,
  hyprland,
  hyprstfu,
  pkgs,
  ...
}: {
  mainMod = "SUPER";

  binds = let
    window = [
      "$mainMod, left, movefocus, l"
      "$mainMod, right, movefocus, r"
      "$mainMod, up, movefocus, u"
      "$mainMod, down, movefocus, d"
    ];

    workspaces = [
      # Go-to
      "$mainMod, 1, workspace, 1"
      "$mainMod, 2, workspace, 2"
      "$mainMod, 3, workspace, 3"
      "$mainMod, 4, workspace, 4"
      "$mainMod, 5, workspace, 5"
      "$mainMod, 6, workspace, 6"
      "$mainMod, 7, workspace, 7"
      "$mainMod, 8, workspace, 8"
      "$mainMod, 9, workspace, 9"
      "$mainMod, 0, workspace, 10"

      # Move window
      "$mainMod SHIFT, 1, movetoworkspace, 1"
      "$mainMod SHIFT, 2, movetoworkspace, 2"
      "$mainMod SHIFT, 3, movetoworkspace, 3"
      "$mainMod SHIFT, 4, movetoworkspace, 4"
      "$mainMod SHIFT, 5, movetoworkspace, 5"
      "$mainMod SHIFT, 6, movetoworkspace, 6"
      "$mainMod SHIFT, 7, movetoworkspace, 7"
      "$mainMod SHIFT, 8, movetoworkspace, 8"
      "$mainMod SHIFT, 9, movetoworkspace, 9"
      "$mainMod SHIFT, 0, movetoworkspace, 10"

      # Resize window
      "$mainMod SHIFT, h, resizeactive, -40 0"
      "$mainMod SHIFT, l, resizeactive, 40 0"
      "$mainMod SHIFT, k, resizeactive, 0 -40"
      "$mainMod SHIFT, j, resizeactive, 0 40"

      # Scroll
      "$mainMod, mouse_down, workspace, e+1"
      "$mainMod, mouse_up, workspace, e-1"

      # Other actions
      "$mainMod, SPACE, togglefloating"
      "$mainMod, MINUS, togglesplit"
      "SUPER_SHIFT, W, killactive"
      "SUPER_SHIFT, MINUS, pseudo"
      "$mainMod, F, fullscreen"
    ];

    custom = let
      rofi-plugin-call = name: program-to-exec: ''${pkgs.rofi}/bin/rofi -modi "${name}:${program-to-exec}" -show ${name}'';
    in [
      {
        "mod+key" = "$mainMod, RETURN";
        "dispatcher" = "exec, ${pkgs.alacritty}/bin/alacritty";
      }
      {
        "mod+key" = ",XF86AudioMicMute";
        "dispatcher" = "exec, ${sys-sound}/bin/sys-sound --toggle-mic";
      }
      {
        "mod+key" = ",XF86AudioMute";
        "dispatcher" = "exec, ${sys-sound}/bin/sys-sound --toggle-vol";
      }
      {
        "mod+key" = ",XF86AudioLowerVolume";
        "dispatcher" = "exec, ${sys-sound}/bin/sys-sound --dec";
      }
      {
        "mod+key" = ",XF86AudioRaiseVolume";
        "dispatcher" = "exec, ${sys-sound}/bin/sys-sound --inc";
      }
      {
        "mod+key" = "SUPER, XF86AudioRaiseVolume";
        "dispatcher" = "exec, ${sys-sound}/bin/sys-sound --inc --unleashed";
      }
      {
        "mod+key" = "SUPER_SHIFT, Print";
        "dispatcher" = "exec, ${grimblast}/bin/grimblast --freeze --notify copy area";
      }
      {
        "mod+key" = "$mainMod, Print";
        "dispatcher" = "exec, ${grimblast}/bin/grimblast --notify copy active";
      }
      {
        "mod+key" = ",Print";
        "dispatcher" = "exec, ${grimblast}/bin/grimblast --notify copy screen";
      }
      {
        "mod+key" = ",XF86MonBrightnessDown";
        "dispatcher" = "exec, ${sys-brightness}/bin/sys-brightness --dec";
      }
      {
        "mod+key" = ",XF86MonBrightnessUp";
        "dispatcher" = "exec, ${sys-brightness}/bin/sys-brightness --inc";
      }
      {
        "mod+key" = "SUPER_SHIFT, C";
        "dispatcher" = "exec, ${rofi-plugin-call "clipboard" "${cliphist-rofi}/bin/cliphist-rofi"}";
      }
      {
        "mod+key" = "SUPER_SHIFT, Q";
        "dispatcher" = "exec, ${pkgs.rofi}/bin/rofi -show window";
      }
      {
        "mod+key" = "$mainMod, Q";
        "dispatcher" = "exec, ${pkgs.rofi}/bin/rofi -show drun";
      }
      {
        "mod+key" = "SUPER_SHIFT, E";
        "dispatcher" = "exec, ${pkgs.bemoji}/bin/bemoji";
      }
      {
        "mod+key" = "SUPER_SHIFT, braceright";
        "dispatcher" = "exec, ${spotify-dbus}/bin/spotify-dbus --next";
      }
      {
        "mod+key" = "SUPER_SHIFT, braceleft";
        "dispatcher" = "exec, ${spotify-dbus}/bin/spotify-dbus --prev";
      }
      {
        "mod+key" = "$mainMod, Pause";
        "dispatcher" = "exec, ${spotify-dbus}/bin/spotify-dbus --toggle";
      }
      {
        "mod+key" = "SUPER_SHIFT, R";
        "dispatcher" = "exec, ${hyprland}/bin/hyprctl reload";
      }
      {
        "mod+key" = "SUPER_SHIFT, S";
        "dispatcher" = "exec, ${pkgs.toybox}/bin/pkill glava || ${pkgs.glava}/bin/glava -d";
      }
      {
        "mod+key" = "$mainMod, M";
        "dispatcher" = "exec, ${hyprstfu}/bin/hyprstfu";
      }
    ];
  in
    window ++ workspaces ++ builtins.map (b: b."mod+key" + ", " + b.dispatcher) custom;

  mouseBinds = [
    # Move/resize windows with mainMod + LMB/RMB and dragging
    "$mainMod, mouse:272, movewindow"
    "$mainMod, mouse:273, resizewindow"
  ];
}

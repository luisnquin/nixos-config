{
  dunstify-brightness,
  dunstify-sound,
  cliphist-rofi,
  spotify-dbus,
  rofi-radio,
  grimblast,
  hyprland,
  pkgs,
  ...
}: {
  bind = let
    rofi-plugin-call = name: program-to-exec: ''${pkgs.rofi}/bin/rofi -modi "${name}:${program-to-exec}" -show ${name}'';
  in [
    "$mainMod, RETURN, exec, ${pkgs.alacritty}/bin/alacritty"

    "$mainMod, SPACE, togglefloating,"
    "$mainMod, MINUS, togglesplit,"
    "SUPER_SHIFT, W, killactive,"
    "SUPER_SHIFT, MINUS, pseudo,"
    "$mainMod, F, fullscreen"
    "$mainMod, M, exit,"

    ",XF86AudioMicMute, exec, ${dunstify-sound}/bin/dunstify-sound --toggle-mic"
    ",XF86AudioMute, exec, ${dunstify-sound}/bin/dunstify-sound --toggle-vol"
    ",XF86AudioLowerVolume, exec, ${dunstify-sound}/bin/dunstify-sound --dec"
    ",XF86AudioRaiseVolume, exec, ${dunstify-sound}/bin/dunstify-sound --inc"

    "SUPER_SHIFT, Print, exec, ${grimblast}/bin/grimblast --freeze --notify copy area"
    "$mainMod, Print, exec, ${grimblast}/bin/grimblast --notify copy active"
    ",Print, exec, ${grimblast}/bin/grimblast --notify copy screen"

    ",XF86MonBrightnessDown, exec, ${dunstify-brightness}/bin/dunstify-brightness --dec"
    ",XF86MonBrightnessUp, exec, ${dunstify-brightness}/bin/dunstify-brightness --inc"

    "SUPER_SHIFT, C, exec, ${rofi-plugin-call "clipboard" "${cliphist-rofi}/bin/cliphist-rofi"}"
    "SUPER_SHIFT, Q, exec, ${pkgs.rofi}/bin/rofi -show window"
    "$mainMod, Q, exec, ${pkgs.rofi}/bin/rofi -show drun"
    "SUPER_SHIFT, Z, exec, ${rofi-radio}/bin/rofi-radio"
    "SUPER_SHIFT, E, exec, ${pkgs.bemoji}/bin/bemoji"

    "SUPER_SHIFT, braceright, exec, ${spotify-dbus}/bin/spotify-dbus --next"
    "SUPER_SHIFT, braceleft, exec, ${spotify-dbus}/bin/spotify-dbus --prev"
    "$mainMod,Pause, exec, ${spotify-dbus}/bin/spotify-dbus --toggle"

    "SUPER_SHIFT, R, exec, ${hyprland}/bin/hyprctl reload"
    "SUPER_SHIFT, S, ${pkgs.toybox}/bin/pkill glava || ${pkgs.glava}/bin/glava -d"

    "$mainMod, left, movefocus, l"
    "$mainMod, right, movefocus, r"
    "$mainMod, up, movefocus, u"
    "$mainMod, down, movefocus, d"

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

    "$mainMod, mouse_down, workspace, e+1"
    "$mainMod, mouse_up, workspace, e-1"
  ];

  bindm = [
    # Move/resize windows with mainMod + LMB/RMB and dragging
    "$mainMod, mouse:272, movewindow"
    "$mainMod, mouse:273, resizewindow"
  ];
}

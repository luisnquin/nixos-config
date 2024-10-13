let
  rulesToWithIdentifier = suffix: list: builtins.map (b: b + ", " + suffix) list;
in {
  general = ''
    layout = dwindle

    gaps_in = 4
    gaps_out = 20
    border_size = 1

    col.active_border = rgba(595959aa)
    col.inactive_border = rgba(595959aa)

    windowrulev2 = float,center,forceinput, class:(Rofi)$
    windowrulev2 = float, title:^(Android Emulator -)
    windowrulev2 = float, title:^(Emulator)$
  '';

  decoration = ''
    rounding = 4
    blur {
        enabled = true
        size = 3
        passes = 1
    }

    active_opacity = 0.9
    inactive_opacity = 1.0
    fullscreen_opacity = 1.0

    drop_shadow = yes
    shadow_range = 4
    shadow_render_power = 3
    col.shadow = rgba(1a1a1aee)
  '';

  animations = ''
    enabled = yes
    bezier = myBezier, 0.05, 0.9, 0.1, 1.05

    animation = windows, 1, 7, myBezier
    animation = windowsOut, 1, 7, default, popin 80%
    animation = border, 1, 10, default
    animation = borderangle, 1, 8, default
    animation = fade, 1, 7, default
    animation = workspaces, 1, 6, default
  '';

  # See https://wiki.hyprland.org/Configuring/Window-Rules/ for more
  rulesv2 =
    (rulesToWithIdentifier "class:^(GLava)$ title:^(GLava)$" [
      "fullscreen"
      "float"
      "noblur"
      "nofocus"
      "noshadow"
      "noborder"
      "pin"
      "move 0 0"
      "size 100% 100%"
      "opacity 0.1"
    ])
    ++ (
      rulesToWithIdentifier "class:^(xwaylandvideobridge)$" [
        "opacity 0.0 override"
        "noinitialfocus"
        "maxsize 1 1"
        "noanim"
        "noblur"
      ]
    )
    ++ (
      rulesToWithIdentifier "class:^(alacritty_hyprdrop)$" [
        "float"
        "center"
        "size 1460 810"
      ]
    );
}

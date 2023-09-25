{pkgs, ...}: {
  home.packages = with pkgs; [xdg-desktop-portal-hyprland];
{
  grimblast,
  pkgs,
  ...
}: {
  home.packages = [
    pkgs.xdg-desktop-portal-hyprland
    grimblast
  ];

  xdg.configFile."hypr/hyprland.conf".text = let
    inherit (pkgs) callPackage;

    spotify-dbus-bin = "${callPackage ./../../../scripts/spotify-dbus {}}/bin/spotify-dbus";
    dunstify-sound-bin = "${callPackage ./../../../scripts/dunstify-sound {barColor = "#ffadbb";}}/bin/dunstify-sound";
    # screen-capture-bin = "${callPackage ./../../../scripts/screen-capture {}}/bin/screen-capture";

    dunstify-brightness = callPackage ./../../../scripts/dunstify-brightness {};
    cliphist-rofi = callPackage ./../../../scripts/cliphist-rofi {};
  in ''
    exec-once=dbus-update-activation-environment --systemd WAYLAND_DISPLAY XDG_CURRENT_DESKTOP

    autogenerated = 0

    # See https://wiki.hyprland.org/Configuring/Monitors/
    monitor=,highrr,auto,1

    # See https://wiki.hyprland.org/Configuring/Keywords/ for more

    # Execute your favorite apps at launch
    # exec-once = waybar & hyprpaper & firefox

    # Source a file (multi-file configs)
    # source = ~/.config/hypr/myColors.conf

    # Some default env vars.
    env = XCURSOR_SIZE,24

    # For all categories, see https://wiki.hyprland.org/Configuring/Variables/
    input {
        numlock_by_default = true
        sensitivity = 0.2

        kb_layout = latam
        kb_variant =
        kb_model =
        kb_options =
        kb_rules =

        follow_mouse = 1

        touchpad {
            natural_scroll = no
            disable_while_typing = no
        }
    }

    general {
        # See https://wiki.hyprland.org/Configuring/Variables/ for more
        layout = dwindle

        gaps_in = 4
        gaps_out = 20
        border_size = 1

        # rgba(33ccffee) rgba(00ff99ee) 45deg
        col.active_border = rgba(595959aa)
        col.inactive_border = rgba(595959aa)
    }

    misc {
        disable_hyprland_logo = true
    }

    decoration {
        # See https://wiki.hyprland.org/Configuring/Variables/ for more
        rounding = 4

        blur {
            enabled = true
            size = 3
            passes = 1
        }

        multisample_edges = true

        active_opacity = 0.9
        inactive_opacity = 1.0
        fullscreen_opacity = 1.0

        drop_shadow = yes
        shadow_range = 4
        shadow_render_power = 3
        col.shadow = rgba(1a1a1aee)
    }

    animations {
        enabled = yes

        # Some default animations, see https://wiki.hyprland.org/Configuring/Animations/ for more

        bezier = myBezier, 0.05, 0.9, 0.1, 1.05

        animation = windows, 1, 7, myBezier
        animation = windowsOut, 1, 7, default, popin 80%
        animation = border, 1, 10, default
        animation = borderangle, 1, 8, default
        animation = fade, 1, 7, default
        animation = workspaces, 1, 6, default
    }

    dwindle {
        # See https://wiki.hyprland.org/Configuring/Dwindle-Layout/ for more
        pseudotile = yes
        preserve_split = yes
    }

    master {
        # See https://wiki.hyprland.org/Configuring/Master-Layout/ for more
        new_is_master = true
    }

    gestures {
        # See https://wiki.hyprland.org/Configuring/Variables/ for more
        workspace_swipe = off
    }

    # Example per-device config
    # See https://wiki.hyprland.org/Configuring/Keywords/#executing for more
    device:epic-mouse-v1 {
        sensitivity = -0.5
    }


    # Example windowrule v1
    # windowrule = float, ^(kitty)$
    # Example windowrule v2
    # windowrulev2 = float,class:^(kitty)$,title:^(kitty)$
    # See https://wiki.hyprland.org/Configuring/Window-Rules/ for more

    exec = pkill waybar & sleep 0.1 && waybar

    # See https://wiki.hyprland.org/Configuring/Keywords/ for more
    $mainMod = SUPER

    # Binds
    bind = $mainMod, RETURN, exec, ${pkgs.alacritty}/bin/alacritty

    bind = $mainMod, SPACE, togglefloating,
    bind = $mainMod, MINUS, togglesplit,
    bind = SUPER_SHIFT, W, killactive,
    bind = SUPER_SHIFT, MINUS, pseudo,
    bind = $mainMod, F, fullscreen
    bind = $mainMod, M, exit,

    bind = ,XF86AudioMicMute, exec, ${dunstify-sound-bin} --toggle-mic
    bind = ,XF86AudioMute, exec, ${dunstify-sound-bin} --toggle-vol
    bind = ,XF86AudioLowerVolume, exec, ${dunstify-sound-bin} --dec
    bind = ,XF86AudioRaiseVolume, exec, ${dunstify-sound-bin} --inc

    bind = ,Print, exec, ${grimblast}/bin/grimblast --notify copy screen
    bind = $mainMod, Print, exec, ${grimblast}/bin/grimblast --notify copy active
    bind = SUPER_SHIFT, Print, exec, ${grimblast}/bin/grimblast --freeze --notify copy area

    bind = ,XF86MonBrightnessDown, exec, ${dunstify-brightness}/bin/dunstify-brightness --dec
    bind = ,XF86MonBrightnessUp, exec, ${dunstify-brightness}/bin/dunstify-brightness --inc

    bind = SUPER_SHIFT, KEY_APOSTROPHE, exec, ${spotify-dbus-bin} --next";
    bind = SUPER_SHIFT, KEY_BACKSLASH, exec, ${spotify-dbus-bin} --prev";
    bind = ,Pause, exec, ${spotify-dbus-bin} --toggle";

    bind = SUPER_SHIFT, R, exec, ${pkgs.hyprland}/bin/hyprctl reload

    bind = SUPER_SHIFT, C, exec, ${pkgs.rofi}/bin/rofi -modi "clipboard:${cliphist-rofi}/bin/cliphist-rofi" -show clipboard
    bind = SUPER_SHIFT, Q, exec, ${pkgs.rofi}/bin/rofi -show window
    bind = $mainMod, Q, exec, ${pkgs.rofi}/bin/rofi -show drun

    # Move focus with mainMod + arrow keys
    bind = $mainMod, left, movefocus, l
    bind = $mainMod, right, movefocus, r
    bind = $mainMod, up, movefocus, u
    bind = $mainMod, down, movefocus, d

    # Switch workspaces with mainMod + [0-9]
    bind = $mainMod, 1, workspace, 1
    bind = $mainMod, 2, workspace, 2
    bind = $mainMod, 3, workspace, 3
    bind = $mainMod, 4, workspace, 4
    bind = $mainMod, 5, workspace, 5
    bind = $mainMod, 6, workspace, 6
    bind = $mainMod, 7, workspace, 7
    bind = $mainMod, 8, workspace, 8
    bind = $mainMod, 9, workspace, 9
    bind = $mainMod, 0, workspace, 10

    # Move active window to a workspace with mainMod + SHIFT + [0-9]
    bind = $mainMod SHIFT, 1, movetoworkspace, 1
    bind = $mainMod SHIFT, 2, movetoworkspace, 2
    bind = $mainMod SHIFT, 3, movetoworkspace, 3
    bind = $mainMod SHIFT, 4, movetoworkspace, 4
    bind = $mainMod SHIFT, 5, movetoworkspace, 5
    bind = $mainMod SHIFT, 6, movetoworkspace, 6
    bind = $mainMod SHIFT, 7, movetoworkspace, 7
    bind = $mainMod SHIFT, 8, movetoworkspace, 8
    bind = $mainMod SHIFT, 9, movetoworkspace, 9
    bind = $mainMod SHIFT, 0, movetoworkspace, 10

    # Scroll through existing workspaces with mainMod + scroll
    bind = $mainMod, mouse_down, workspace, e+1
    bind = $mainMod, mouse_up, workspace, e-1

    # Move/resize windows with mainMod + LMB/RMB and dragging
    bindm = $mainMod, mouse:272, movewindow
    bindm = $mainMod, mouse:273, resizewindow
  '';
}

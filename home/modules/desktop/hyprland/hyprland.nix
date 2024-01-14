{
  dunstify-brightness,
  dunstify-sound,
  cliphist-rofi,
  spotify-dbus,
  rofi-radio,
  grimblast,
  hyprland,
  pkgs,
  lib,
  ...
}: let
  # ugly, improve it
  binds = import ./binds.nix {
    inherit
      dunstify-brightness
      dunstify-sound
      cliphist-rofi
      spotify-dbus
      rofi-radio
      grimblast
      hyprland
      pkgs
      ;
  };

  window = import ./window.nix;

  # https://wiki.hyprland.org/Configuring/Keywords/
  mainMod = "SUPER";

  exec = [
    "pkill waybar & sleep 0.1 && waybar"
  ];

  exec-once = [
    "${pkgs.wl-clipboard}/bin/wl-paste --watch ${pkgs.cliphist}/bin/cliphist store -max-items 200"
    "dbus-update-activation-environment --systemd WAYLAND_DISPLAY XDG_CURRENT_DESKTOP"
  ];

  rules = ''
    autogenerated = 0
    monitor=,highrr,auto,1
    env = XCURSOR_SIZE,24

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

    misc {
        disable_hyprland_logo = true
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
        workspace_swipe = off
    }

    device:epic-mouse-v1 {
        sensitivity = -0.5
    }
  '';

  listToPrefixedLines = prefix: list: lib.strings.concatMapStrings (b: prefix + b + "\n") list;
in {
  # More keysyms here: https://github.com/xkbcommon/libxkbcommon/blob/master/include/xkbcommon/xkbcommon-keysyms.h
  xdg.configFile."hypr/hyprland.conf".text = ''
    ${listToPrefixedLines "exec-once = " exec-once}

    ${rules}
    $mainMod = ${mainMod}

    general {
      ${window.general}
    }

    decoration {
      ${window.decoration}
    }

    animations {
      ${window.animations}
    }

    ${listToPrefixedLines "windowrulev2 = " window.rulesv2}

    ${listToPrefixedLines "exec = " exec}
    ${listToPrefixedLines "bind = " binds.bind}
    ${listToPrefixedLines "bindm = " binds.bindm}
  '';
}

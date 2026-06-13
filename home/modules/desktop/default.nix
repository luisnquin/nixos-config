{
  imports = [
    ./hyprland
    ./waybar
    ./eww
    ./awww
    ./fuzzel.nix

    ./clipboard.nix
    ./color-picker.nix
    ./gtk.nix
    ./mako.nix
    ./mouse.nix
    ./notifications.nix
    ./raffi.nix
    ./xdg.nix
  ];

  xsession.enable = true;
}

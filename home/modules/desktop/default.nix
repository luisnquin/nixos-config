{
  imports = [
    ./hyprland
    ./waybar
    ./eww
    ./heft.nix
    ./awww
    ./fuzzel.nix
    ./herdr.nix

    ./clipboard.nix
    ./color-picker.nix
    ./gtk.nix
    ./mako.nix
    ./mouse.nix
    ./notifications.nix
    ./raffi.nix
    ./wayvnc.nix
    ./xdg.nix
  ];

  xsession.enable = true;
}

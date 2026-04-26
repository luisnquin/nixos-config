{
  imports = [
    ./hyprland
    ./waybar
    ./awww
    ./fuzzel.nix

    ./color-picker.nix
    ./gsimplecal.nix
    ./gtk.nix
    ./mako.nix
    ./mouse.nix
    ./notifications.nix
    ./raffi.nix
    ./xdg.nix
  ];

  xsession.enable = true;
}

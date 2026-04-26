{
  imports = [
    ./hyprland
    ./waybar
    ./awww
    ./rofi

    ./color-picker.nix
    ./gsimplecal.nix
    ./gtk.nix
    ./hyprlauncher.nix
    ./mako.nix
    ./mouse.nix
    ./notifications.nix
    ./raffi.nix
    ./xdg.nix
  ];

  xsession.enable = true;
}

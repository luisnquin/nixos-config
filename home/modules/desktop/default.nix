{
  imports = [
    ./hyprland
    ./waybar
    ./swww
    ./rofi

    ./asshole.nix
    ./color-picker.nix
    ./dunst.nix
    ./gsimplecal.nix
    ./gtk.nix
    ./mako.nix
    ./mouse.nix
    ./notifications.nix
    ./xdg.nix
  ];

  xsession.enable = true;
}

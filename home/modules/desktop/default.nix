{
  imports = [
    ./hyprland
    ./waybar
    ./swww

    ./asshole.nix
    ./color-picker.nix
    ./dunst.nix
    ./gsimplecal.nix
    ./gtk.nix
    ./mako.nix
    ./mouse.nix
    ./notifications.nix
    ./rofi.nix
    ./xdg.nix
  ];

  xsession.enable = true;
}

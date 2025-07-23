{
  imports = [
    ./wayland/hyprland
    ./color-picker.nix
    ./notifications.nix
    ./gsimplecal.nix
    ./xdg.nix
  ];

  xsession.enable = true;
}

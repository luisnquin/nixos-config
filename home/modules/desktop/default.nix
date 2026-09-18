{
  imports = [
    ./hyprland
    ./shell
    ./heft.nix
    ./awww
    ./herdr.nix
    ./vicinae.nix

    ./color-picker.nix
    ./gtk.nix
    ./mako.nix
    ./mouse.nix
    ./notifications.nix
    ./wayvnc.nix
    ./xdg.nix
  ];

  xsession.enable = true;
}

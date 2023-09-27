{host, ...}: let
  desktopModule =
    {
      "hyprland" = ./hyprland.nix;
      "plasma" = ./plasma.nix;
      "i3" = ./i3.nix;
    }
    .${host.desktop};
in {
  imports = [
    desktopModule
    ./common.nix
  ];
}

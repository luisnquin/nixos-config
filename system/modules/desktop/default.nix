{host, ...}: let
  desktopModule =
    {
      "plasma" = ./plasma.nix;
      "hypr" = ./hypr.nix;
      "i3" = ./i3wm.nix;
    }
    .${host.desktop};
in {
  imports = [
    desktopModule
  ];
}

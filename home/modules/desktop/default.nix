{host, ...}: let
  desktopModule =
    {
      "plasma" = ./plasma;
      "hrpr" = ./hypr;
      "i3" = ./i3wm;
    }
    .${host.desktop};
in {
  imports = [
    desktopModule
  ];
}

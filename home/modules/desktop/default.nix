{host, ...}: let
  desktopModules =
    {
      "plasma" = [
        ./plasma
      ];
      "hypr" = [
        ./hypr
      ];
      "i3" = [
        ./i3wm
      ];
    }
    .${host.desktop};
in {
  imports = desktopModules;
}

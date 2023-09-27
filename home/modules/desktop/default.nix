{host, ...}: let
  desktopModules =
    {
      "plasma" = [
        ./plasma
      ];
      "hyprland" = [
        ./hyprland
      ];
      "i3" = [
        ./i3
      ];
    }
    .${host.desktop};
in {
  imports = desktopModules;

  xsession.enable = true;
}

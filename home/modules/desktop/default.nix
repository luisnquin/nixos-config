{host, ...}: let
  desktopModules =
    {
      "plasma" = [
        ./plasma
      ];
      "hypr" = [
        ./hypr
        ./greenclip.nix
      ];
      "i3" = [
        ./i3wm
        ./greenclip.nix
      ];
    }
    .${host.desktop};
in {
  imports = desktopModules;
}

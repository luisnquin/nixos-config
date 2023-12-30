{
  xdg-desktop-portal-hyprland,
  hyprland,
  pkgs,
  ...
}: {
  environment.pathsToLink = ["/libexec"];

  programs = {
    hyprland = {
      enable = true;
      package = hyprland;
      # enableNvidiaPatches = true;
      portalPackage = xdg-desktop-portal-hyprland;
    };

    xwayland.enable = true;
  };
}

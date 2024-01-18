{
  xdg-desktop-portal-hyprland,
  hyprland,
  pkgs,
  ...
}: {
  environment.pathsToLink = ["/libexec"];

  xdg.portal.extraPortals = [pkgs.xdg-desktop-portal-wlr];

  programs = {
    hyprland = {
      enable = true;
      package = hyprland;
      portalPackage = xdg-desktop-portal-hyprland;
    };

    xwayland.enable = true;
  };
}

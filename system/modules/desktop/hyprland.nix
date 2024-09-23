{pkgs, ...}: {
  environment.pathsToLink = ["/libexec"];

  xdg.portal.extraPortals = [pkgs.xdg-desktop-portal-wlr];

  programs = {
    hyprland = {
      enable = true;
      package = pkgs.hyprland;
      portalPackage = pkgs.xdg-desktop-portal-hyprland;
    };

    xwayland.enable = true;
  };
}

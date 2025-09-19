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

  nix.settings = {
    trusted-public-keys = [
      "hyprland.cachix.org-1:a7pgxzMz7+chwVL3/pzj6jIBMioiJM7ypFP8PwtkuGc="
    ];

    substituters = [
      "https://hyprland.cachix.org"
    ];
  };
}

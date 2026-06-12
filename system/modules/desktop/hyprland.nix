{
  hmConfig,
  pkgs,
  ...
}: {
  environment.pathsToLink = ["/libexec"];

  xdg.portal = {
    # xdgOpenUsePortal = true;
    extraPortals = with pkgs; [
      pkgs.xdg-desktop-portal-gtk
      kdePackages.xdg-desktop-portal-kde
    ];

    configPackages = [
      hmConfig.wayland.windowManager.hyprland.package
    ];

    config.hyprland = {
      default = ["hyprland" "gtk"];
      "org.freedesktop.impl.portal.FileChooser" = "kde";
      "org.freedesktop.impl.portal.Print" = "kde";
    };
  };

  programs = {
    hyprland = {
      enable = true;
      package = pkgs.hyprland;
      portalPackage = pkgs.xdg-desktop-portal-hyprland;
    };

    xwayland.enable = true;
  };
}

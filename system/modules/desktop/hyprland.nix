{
  xdg-desktop-portal-hyprland,
  hyprland,
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

{
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
      portalPackage = pkgs.xdg-desktop-portal-hyprland;
    };

    xwayland.enable = true;
  };
}

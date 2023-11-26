{hyprland, ...}: {
  environment.pathsToLink = ["/libexec"];

  programs = {
    xwayland.enable = true;

    hyprland = {
      enable = true;
      package = hyprland;
      enableNvidiaPatches = true;
    };
  };
}

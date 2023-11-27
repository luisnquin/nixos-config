{hyprland, ...}: {
  environment.pathsToLink = ["/libexec"];

  programs.xwayland.enable = true;

  programs = {
    hyprland = {
      enable = true;
      package = hyprland;
      enableNvidiaPatches = true;
    };
  };
}

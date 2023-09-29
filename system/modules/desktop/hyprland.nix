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

  # services.xserver.displayManager = {
  #   defaultSession = "hyprland";

  #   session = [
  #     {
  #       manage = "desktop";
  #       name = "hyprland";
  #       start = ''exec $HOME/.xsession'';
  #     }
  #   ];
  # };
}
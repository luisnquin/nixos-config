{pkgs, ...}: {
  environment.pathsToLink = ["/libexec"];

  services.xserver = {
    displayManager.defaultSession = "none+i3";

    windowManager.i3 = {
      enable = true;

      extraPackages = with pkgs; [
        numlockx
        nitrogen
        i3blocks
        i3lock
        clipit
      ];
    };
  };
}

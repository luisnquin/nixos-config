{
  pkgs,
  host,
  ...
}: {
  environment.pathsToLink = ["/libexec"];
  programs.dconf.enable = true;

  services.xserver = {
    enable = true;
    autorun = true;
    layout = host.keyboardLayout;
    libinput.enable = true;

    displayManager = {
      defaultSession = "none+i3";
      gdm = {
        enable = true;
        autoSuspend = false;
      };
    };

    desktopManager = {
      xterm.enable = true;
    };

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

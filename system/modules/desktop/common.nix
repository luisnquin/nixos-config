{
  pkgs,
  host,
  ...
}: {
  programs.dconf.enable = true;

  services.xserver = {
    enable = true;
    autorun = true;
    layout = host.keyboardLayout;
    libinput = {
      enable = true;

      touchpad = {
        tapping = true;
        naturalScrolling = true;
        middleEmulation = true;
      };
    };

    displayManager.gdm = {
      enable = true;
      autoSuspend = false;
    };

    desktopManager.xterm.enable = true;
  };

  # xdg.portal.extraPortals = [pkgs.xdg-desktop-portal-gtk];
}

{
  isTiling,
  config,
  pkgs,
  host,
  lib,
  ...
}: {
  programs.dconf.enable = true;

  services = {
    xserver = {
      enable = true;
      autorun = true;
      xkb.layout = host.keyboardLayout;

      # displayManager.gdm = {
      #   enable = true;
      #   autoSuspend = false;
      #   wayland = true;
      # };

      displayManager.startx.enable = true;
      desktopManager.xterm.enable = true;
    };

    libinput = {
      enable = true;

      touchpad = {
        tapping = true;
        naturalScrolling = true;
        middleEmulation = true;
      };
    };
  };

  programs.kdeconnect.enable = true;

  xdg.portal.extraPortals = lib.mkIf (isTiling && config.programs.kdeconnect.enable) [
    pkgs.libsForQt5.xdg-desktop-portal-kde
  ];
}

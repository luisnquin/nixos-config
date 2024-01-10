{
  isTiling,
  config,
  pkgs,
  host,
  lib,
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
      wayland = true;
    };

    desktopManager.xterm.enable = true;
  };

  programs.kdeconnect.enable = true;

  xdg.portal.extraPortals = lib.mkIf (isTiling && config.programs.kdeconnect.enable) [
    pkgs.libsForQt5.xdg-desktop-portal-kde
  ];
}

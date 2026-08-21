{
  isTiling,
  config,
  host,
  pkgs,
  lib,
  ...
}: {
  imports = [
    ./clipboard.nix
    ./hyprland.nix
    ./fonts
    ./thunar.nix
  ];

  programs.dconf.enable = true;
  programs.wayvnc.enable = true;

  programs.ttygate = {
    enable = true;
    sessionCmd = ["${pkgs.hyprland}/bin/start-hyprland"];
    accent = "amber";
    idleStatus = "AWAITING IDENTIFICATION";
    defaultUser = "luisnquin";
    journalUser = "greeter";
  };

  services = {
    greetd = {
      enable = true;
      settings = {
        default_session = {
          command = lib.getExe config.programs.ttygate.package;
        };
      };
    };

    xserver = {
      enable = true;
      autorun = true;
      xkb.layout = host.keyboardLayout;
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
    pkgs.kdePackages.xdg-desktop-portal-kde
  ];
}

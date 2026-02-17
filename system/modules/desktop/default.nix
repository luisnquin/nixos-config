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
    ./fonts.nix
    ./thunar.nix
  ];

  programs.dconf.enable = true;

  services = {
    greetd = {
      enable = true;
      settings = {
        default_session = {
          command = ''${pkgs.greetd}/bin/agreety --cmd ${pkgs.hyprland}/bin/start-hyprland'';
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

{pkgs, ...}: {
  environment.systemPackages = with pkgs; [
    lightly-qt # KDE windows style
    flat-remix-gtk # KDE theme
    tela-icon-theme
  ];

  # TODO: wayland + sway
  # xdg.portal.wlr.enable
  services = {
    xserver = {
      videoDrivers = ["nvidia"];
      libinput.enable = true;
      layout = "latam";
      autorun = true;
      enable = true;

      displayManager = {
        gdm = {
          enable = true;
          autoSuspend = false;
        };

        startx.enable = true;
        defaultSession = "plasma";
      };

      desktopManager = {
        plasma5 = {
          enable = true;
          runUsingSystemd = true;
        };

        xterm.enable = true;
      };
    };
  };
}

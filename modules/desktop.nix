{
  config,
  pkgs,
  ...
}: {
  environment.systemPackages = with pkgs; [
    lightly-qt # KDE style
    flat-remix-gtk # I'm not sure to use this
    tela-icon-theme
  ];

  services.xserver = {
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
}

{
  config,
  pkgs,
  ...
}: {
  environment.systemPackages = with pkgs; [
    lightly-qt # KDE windows style
    flat-remix-gtk # KDE theme
    tela-icon-theme
  ];

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

    redshift = {
      enable = true;
      temperature = {
        day = 5700;
        night = 5300;
      };

      brightness = {
        day = "1";
        night = "1";
      };

      # extraOptions = [# Fake location "-l 55.7:12.6"];
    };
  };
}

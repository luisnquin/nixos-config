{
  pkgs,
  host,
  ...
}: {
  services = let
    settings = {
      i3wm = {
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

      plasma = {
        displayManager = {
          defaultSession = "plasma";
          startx.enable = true;
          gdm = {
            enable = true;
            autoSuspend = false;
          };
        };

        desktopManager = {
          xterm.enable = true;

          plasma5 = {
            enable = true;
            runUsingSystemd = true;
          };
        };
      };
    };
  in {
    xserver =
      {
        enable = true;
        autorun = true;
        layout = "latam";
        libinput.enable = true;
      }
      // (
        if host.usePlasma
        then settings.plasma
        else settings.i3wm
      );
  };

  environment =
    if host.usePlasma
    then {
      systemPackages = with pkgs; [
        lightly-qt
        flat-remix-gtk
        tela-icon-theme
      ];
    }
    else {
      pathsToLink = ["/libexec"];
    };

  programs.dconf.enable = true;
}

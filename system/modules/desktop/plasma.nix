{
  pkgs,
  host,
  ...
}: {
  environment.systemPackages = with pkgs; [
    lightly-qt
    flat-remix-gtk
    tela-icon-theme
  ];

  programs.dconf.enable = true;

  services.xserver = {
    enable = true;
    autorun = true;
    layout = host.keyboardLayout;
    libinput.enable = true;

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
}

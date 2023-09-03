{pkgs, ...}: {
  environment.systemPackages = with pkgs; [
    lightly-qt
    flat-remix-gtk
    tela-icon-theme
  ];

  services.xserver = {
    displayManager = {
      defaultSession = "plasma";
      startx.enable = true;
    };

    desktopManager.plasma5 = {
      enable = true;
      runUsingSystemd = true;
    };
  };
}

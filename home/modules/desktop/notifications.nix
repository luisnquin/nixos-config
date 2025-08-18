{pkgs, ...}: {
  home.packages = [
    pkgs.libnotify
  ];

  services.kdeconnect = {
    enable = true;
    indicator = true;
  };
}

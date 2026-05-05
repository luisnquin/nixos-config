{pkgs, ...}: {
  services = {
    librespot = {
      enable = true;
      args = [
        "-n"
        "Librespot Speaker"
        "-b"
        "160"
        "--enable-oauth"
      ];
      settings = {
        initial-volume = 100;
      };
    };
    playerctld.enable = true;
  };

  home.packages = [
    pkgs.spotify-qt
  ];
}

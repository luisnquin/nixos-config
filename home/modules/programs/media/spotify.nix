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
    };
    playerctld.enable = true;
  };

  home.packages = [
    pkgs.spotify-qt
  ];
}

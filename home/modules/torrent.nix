{pkgs, ...}: {
  home.packages = [
    pkgs.fragments # Graphical client to download torrents
  ];
}

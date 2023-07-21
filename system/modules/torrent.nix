{pkgs, ...}: {
  environment.systemPackages = with pkgs; [
    fragments # Graphical client to download torrents
  ];
}

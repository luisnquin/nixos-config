{
  config,
  pkgs,
  ...
}: let
  transmissionPort = 9091;
in {
  services.transmission = {
    enable = true;
    user = "transmission";
    group = "transmission";

    settings = {
      # Micro Transport Protocol that allows P2P connections
      utp-enabled = true;

      # to be written by whoever
      umask = 0;

      # whatever I'm a lonely bottleneck
      rpc-bind-address = "127.0.0.1";
      rpc-port = transmissionPort;

      openFirewall = true;
      peer-port-random-on-start = true;
      # Dynamic ports
      peer-port-random-low = 49152;
      peer-port-random-high = 65535;

      # FS
      watch-dir-enabled = false;
      incomplete-dir-enabled = true;
    };
  };

  # TODO
  # services.transmission.settings.script-torrent-done-enabled
  # services.transmission.settings.script-torrent-done-filename

  # TODO: add https://github.com/PanAeon/transg-tui
  environment.systemPackages = with pkgs; [
    transmission
    qbittorrent
    fragments # Graphical client to download torrents
  ];
}

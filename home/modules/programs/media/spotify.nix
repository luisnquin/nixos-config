{
  nixosConfig,
  pkgs,
  ...
}: {
  services = {
    go-librespot = {
      enable = true;
      settings = {
        log_level = "info";
        device_name = nixosConfig.networking.hostName;
        device_type = "speaker";
        audio_backend = "pulseaudio";

        zeroconf_enabled = false;

        credentials = {
          type = "interactive";
        };

        bitrate = 160;
        initial_volume = 100;
        ignore_last_volume = true;
        disable_autoplay = false;
        mpris_enabled = true;
      };
    };

    playerctld.enable = true;
  };

  home.packages = [
    pkgs.spotify-qt
  ];
}
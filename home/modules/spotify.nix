{
  nixosConfig,
  spicetify,
  libx,
  host,
  pkgs,
  ...
}: {
  programs = {
    spicetify = with spicetify; {
      enable = true;
      theme = themes.text;

      spotifyPackage = pkgs.spotify;

      enabledExtensions = with extensions; [
        historyShortcut
        fullAppDisplay
        fullAlbumDate
        hidePodcasts
        popupLyrics
        songStats
        skipStats
        shuffle
        wikify
        # genre
      ];
    };

    # Try to configure it to use vi keys
    # https://github.com/aome510/spotify-player/blob/master/docs/config.md
    spotify-player = {
      enable = true;
      package = pkgs.spotify-player;
    };
  };

  home.shellAliases = let
    inherit (pkgs) lib;
  in {
    spt = lib.getExe pkgs.spotify-player;
  };

  services.spotifyd = {
    enable = true;
    settings.global = rec {
      username = libx.base64.decode "eWVzZWxvbnk=";
      password_cmd = "cat ${nixosConfig.age.secrets.spotify-access-secret.path}";
      use_keyring = false;
      use_mpris = true;
      dbus_type = "session";
      backend = "pulseaudio";
      device = "pipewire";
      control = device;
      audio_format = "S16";
      mixer = "PCM";
      volume_controller = "none";
      device_name = "${host.name}-systemd";
      bitrate = 160;
      # cache_path = "/etc/spotifyd";
      # max_cache_size = let GB = 1000000000; in GB;
      no_audio_cache = true;
      initial_volume = "90";
      autoplay = true;
      volume_normalisation = true;
      normalisation_pregain = -10;
      device_type = "computer";
    };
  };
}

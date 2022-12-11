{
  config,
  username,
  ...
}: {
  systemd.services.spotifyd = {
    after = ["network-online.target" "sound.target"];
    serviceConfig = {
      SupplementaryGroups = ["audio"];
      DynamicUser = true;
      Restart = "always";
    };
  };

  services.spotifyd = {
    enable = true;
    settings.global = {
      # use_keyring = true;
      autoplay = true;
      # backend = "pulseaudio";
      bitrate = 320;
      # TODO: pass username through the parent module
      # cache_path = ''/home/${username}/.cache/spotifyd'';
      # dbus_type = "session";
      device_name = config.networking.hostName;
      device_type = "computer";
      initial_volume = "80";
      max_cache_size = 1000000000;
      no_audio_cache = false;
      # TODO: implementation for secrets and other personal settings
      password = "";
      # use_mpris = true;
      username = "yeselony"; # Thanks to the guy who stole and changed my username 5 years ago
      volume_normalisation = false;
    };
  };
}

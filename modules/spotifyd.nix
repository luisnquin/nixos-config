{
  config,
  username,
  lib,
  ...
}
: let
  owner = import "/etc/nixos/owner.nix";
in {
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

    # Ref: http://spotifyd.github.io/spotifyd/config/File.html
    settings.global = {
      device = "hw:0,0"; # 'aplay -l' or 'aplay -L'
      control = "hw:0,0"; # Like the sugar
      device_name = config.networking.hostName;
      cache_path = ''/home/${owner.username}/.cache/spotifyd'';
      max_cache_size = 1000000000;
      volume_normalisation = false;
      device_type = "speaker";
      no_audio_cache = false;
      initial_volume = "80";
      dbus_type = "session"; # this scope should be enough
      use_mpris = true;
      backend = "alsa"; # I'm using pipewire, so
      autoplay = true;
      bitrate = 320;

      username = owner.spotifyUsername;
      password = owner.spotifyPassword;
    };
  };
}

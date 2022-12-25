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
      device = "hw:0,0";
      device_name = config.networking.hostName;
      max_cache_size = 1000000000;
      volume_normalisation = false;
      device_type = "computer";
      no_audio_cache = false;
      initial_volume = "80";
      backend = "alsa";
      autoplay = true;
      bitrate = 320;

      username = owner.spotifyUsername;
      password = owner.spotifyPassword;
    };
  };
}
# cache_path = ''/home/${username}/.cache/spotifyd'';
# dbus_type = "session";
# use_keyring = true;
# use_mpris = true;


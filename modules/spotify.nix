{
  config,
  pkgs,
  #  lib,
  ...
}
: let
  owner = import "/etc/nixos/owner.nix";
in {
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

  systemd.services.spotifyd = {
    after = ["network-online.target" "sound.target"];
    serviceConfig = {
      SupplementaryGroups = ["audio"];
      DynamicUser = true;
      Restart = "always";
    };
  };

  environment.systemPackages = with pkgs; [
    # spicetify-cli
    spotify-tui
    spotifyd
    spotify
  ];
}
/*
home-manager.users."${owner.username}" = {
  xdg.configFile."spotify-tui/config.yml" = {
    source = lib.yamlGenerate "spotify-tui-config" {
      theme = let
        color = {
          skyBlue = "72, 186, 224";
          white = "216, 219, 226"; # pretty white
          hotPink = "222, 22, 82";
          deadRed = "130, 78, 84";
          friendlyRed = "219, 39, 60";
          drunkYellow = "222, 200, 31";
          pinkishWhite = "232, 218, 222";
          stiffYellow = "237, 177, 26";
          deadSkyBlue = "88, 164, 176";
          bluishWhite = "169, 188, 208";
          bluishGray = "89, 98, 117";
        };
      in {
        active = "${color.skyBlue}";
        inactive = "${color.white}";
        banner = "${color.hotPink}";

        error_border = "${color.deadRed}";
        error_text = "${color.friendlyRed}";
        hint = "${color.drunkYellow}";

        playbar_background = "${color.pinkishWhite}";
        playbar_progress = "${color.hotPink}";
        playbar_progress_text = "${color.pinkishWhite}";
        playbar_text = "${color.pinkishWhite}";

        hovered = "${color.stiffYellow}";
        selected = "${color.deadSkyBlue}";

        text = "${color.bluishWhite}";
        header = "${color.bluishGray}";
      };

      behavior = {
        seek_milliseconds = 5000;
        volume_increment = 10;

        tick_rate_milliseconds = 350;
        enable_text_emphasis = true;
        show_loading_indicator = true;
        enforce_wide_search_bar = false;

        liked_icon = "♥";
        shuffle_icon = "咽";
        repeat_track_icon = "綾";
        repeat_context_icon = "凌";
        playing_icon = "▶";
        paused_icon = "⏸";

        set_window_title = true;
      };

      keybindings = {
        back = "ctrl-q";
        jump_to_album = "a";
        jump_to_artist_album = "A";

        manage_devices = "d";
        decrease_volume = "-";
        increase_volume = "+";
        toggle_playback = " ";
        seek_backwards = "<";
        seek_forwards = ">";
        next_track = "n";
        previous_track = "p";
        copy_song_url = "c";
        copy_album_url = "C";
        help = "?";
        shuffle = "ctrl-s";
        repeat = "r";
        search = "/";
        audio_analysis = "v";
        jump_to_context = "o";
        basic_view = "B";
        add_item_to_queue = "z";
      };
    };
  };
};
*/


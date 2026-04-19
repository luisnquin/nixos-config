{
  config,
  lib,
  pkgs,
  ...
}: let
  spotatuiClass = "spotatui";
  spotatuiSession = pkgs.writeShellApplication {
    name = "spotatui-session";
    runtimeInputs = [
      pkgs.spotatui
      pkgs.util-linux
    ];
    text = ''
      lock_dir="''${XDG_RUNTIME_DIR:-/tmp}"
      lock_file="$lock_dir/spotatui-runtime.lock"

      exec 8>"$lock_file"
      flock -n 8 || exit 0

      exec spotatui
    '';
  };
  spotatuiLauncher = pkgs.writeShellApplication {
    name = "spotatui-launcher";
    runtimeInputs = [
      config.programs.ghostty.package
      pkgs.coreutils
      pkgs.hyprland
      pkgs.spotatui
      pkgs.jq
      pkgs.procps
      pkgs.util-linux
    ];
    text = ''
      class="${spotatuiClass}"
      lock_dir="''${XDG_RUNTIME_DIR:-/tmp}"
      lock_file="$lock_dir/spotatui-launcher.lock"

      focus_spotatui() {
        clients_json="$(hyprctl clients -j 2>/dev/null || true)"
        address=""

        if [ -n "$clients_json" ]; then
          address="$(
            printf '%s\n' "$clients_json" \
              | jq -r --arg class "$class" 'first(.[] | select(.class == $class) | .address) // empty'
          )"
        fi

        if [ -n "$address" ]; then
          hyprctl dispatch focuswindow "address:$address"
          return 0
        fi

        return 1
      }

      focus_spotatui && exit 0

      if pgrep -u "$(id -u)" -x spotatui >/dev/null; then
        focus_spotatui || true
        exit 0
      fi

      exec 9>"$lock_file"
      if ! flock -n 9; then
        attempts=0
        while [ "$attempts" -lt 20 ]; do
          focus_spotatui && exit 0
          attempts=$((attempts + 1))
          sleep 0.1
        done

        exit 0
      fi

      focus_spotatui && exit 0

      exec ghostty --class="$class" -e ${lib.getExe spotatuiSession}
    '';
  };
  yamlFormat = pkgs.formats.yaml {};
in {
  home.packages = [
    pkgs.spotatui
  ];

  xdg.desktopEntries.spotatui = {
    name = "spotatui";
    type = "Application";
    comment = "Spotify terminal client";
    exec = lib.getExe spotatuiLauncher;
    terminal = false;
    categories = ["Audio" "Music" "Player"];
    startupNotify = false;
  };

  # https://github.com/LargeModGames/spotatui/wiki/Configuration#configuration
  xdg.configFile."spotatui/config.yml".source = yamlFormat.generate "spotatui-config.yml" {
    keybindings = {
      back = "q";
      next_page = "ctrl-d";
      previous_page = "ctrl-u";
      jump_to_start = "ctrl-a";
      jump_to_end = "ctrl-e";
      jump_to_album = "a";
      jump_to_artist_album = "A";
      jump_to_context = "o";
      manage_devices = "d";
      decrease_volume = "-";
      increase_volume = "+";
      toggle_playback = "space";
      seek_backwards = "<";
      seek_forwards = ">";
      next_track = "n";
      previous_track = "p";
      force_previous_track = "P";
      help = "?";
      shuffle = "ctrl-s";
      repeat = "ctrl-r";
      search = "/";
      submit = "enter";
      copy_song_url = "c";
      copy_album_url = "C";
      audio_analysis = "v";
      lyrics_view = "B";
      cover_art_view = "G";
      add_item_to_queue = "z";
      show_queue = "Q";
      open_settings = "alt-,";
      save_settings = "alt-s";
      listening_party = "ctrl-p";
    };

    behavior = {
      seek_milliseconds = 5000;
      volume_increment = 10;
      volume_percent = 90;
      tick_rate_milliseconds = 16;
      enable_text_emphasis = true;
      show_loading_indicator = true;
      enforce_wide_search_bar = false;
      enable_global_song_count = true;
      enable_discord_rpc = true;
      discord_rpc_client_id = null;
      enable_announcements = true;
      announcement_feed_url = null;
      seen_announcement_ids = [
        "2026-02-27-major-refactor-complete"
      ];
      shuffle_enabled = false;
      liked_icon = "♥";
      shuffle_icon = "🔀";
      repeat_track_icon = "🔂";
      repeat_context_icon = "🔁";
      playing_icon = "▶";
      paused_icon = "⏸";
      set_window_title = true;
      visualizer_style = "Equalizer";
      dismissed_announcements = [];
      relay_server_url = "wss://spotatui-party.spotatui.workers.dev/ws";
      stop_after_current_track = false;
      sidebar_width_percent = 25;
      playbar_height_rows = 6;
      library_height_percent = 30;
      startup_behavior = "continue";
      disable_auto_update = false;
      auto_update_delay = "0";
    };

    theme = {
      active = "0, 180, 180";
      banner = "0, 200, 200";
      error_border = "200, 0, 0";
      error_text = "255, 100, 100";
      hint = "200, 200, 0";
      hovered = "180, 0, 180";
      inactive = "128, 128, 128";
      playbar_background = "Reset";
      playbar_progress = "0, 200, 200";
      playbar_progress_text = "255, 255, 255";
      playbar_text = "Reset";
      selected = "0, 200, 200";
      text = "Reset";
      background = "Reset";
      header = "Reset";
      highlighted_lyrics = "0, 200, 200";
    };
  };

  services.playerctld.enable = true;
}

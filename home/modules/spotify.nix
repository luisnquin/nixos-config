{
  spicetify,
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
      alias = "spt";
    };
  };
}

{
  spicetify,
  pkgs,
  ...
}: {
  programs.spicetify = with spicetify; {
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
      genre
    ];
  };
}

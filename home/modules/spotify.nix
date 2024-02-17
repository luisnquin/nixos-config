{
  pkgs-latest,
  spicetify,
  ...
}: {
  programs.spicetify = with spicetify; {
    enable = true;
    theme = themes.text;

    spotifyPackage = pkgs-latest.spotify;

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

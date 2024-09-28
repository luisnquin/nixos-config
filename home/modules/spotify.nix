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
      volumePercentage
      beautifulLyrics
      historyShortcut
      fullAppDisplay
      fullAlbumDate
      betterGenres
      hidePodcasts
      popupLyrics
      songStats
      skipStats
      shuffle
      wikify
    ];
  };
}

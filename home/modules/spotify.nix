{spicetify, ...}: {
  programs.spicetify = with spicetify; {
    enable = true;
    theme = themes.text;

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

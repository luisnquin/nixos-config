{
  inputs,
  system,
  pkgs,
  ...
}: let
  spicetify = inputs.spicetify-nix.legacyPackages.${system};
in {
  programs.spicetify = with spicetify; {
    enable = true;
    theme = themes.hazy;

    spotifyPackage = pkgs.spotify;

    enabledExtensions = with extensions; [
      volumePercentage
      historyShortcut
      fullAppDisplay
      aiBandBlocker
      fullAlbumDate
      betterGenres
      hidePodcasts
      popupLyrics
      songStats
      skipStats
      trashbin
      powerBar
      adblock
      shuffle
      wikify
      oneko
    ];
  };

  services.playerctld.enable = true;
}

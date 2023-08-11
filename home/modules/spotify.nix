{
  spicetify-nix,
  pkgs,
  lib,
  ...
}
: let
  spicePkgs = spicetify-nix.packages.${pkgs.system}.default;
in {
  imports = [spicetify-nix.homeManagerModule];

  programs.spicetify = with spicePkgs; {
    enable = true;
    theme = themes."Sleek";
    colorScheme = "cherry";
    # theme = themes.Ziro;
    # colorScheme = "ylx-ui";

    enabledExtensions = with extensions; [
      historyShortcut
      fullAppDisplay
      fullAlbumDate
      hidePodcasts
      popupLyrics
      songStats
      skipStats
      trashbin
      shuffle
      wikify
      genre
    ];
  };
}

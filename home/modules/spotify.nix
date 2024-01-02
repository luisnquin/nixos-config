{
  spicetify-nix,
  pkgs,
  ...
}
: let
  spicePkgs = spicetify-nix.packages.${pkgs.system}.default;
in {
  imports = [spicetify-nix.homeManagerModule];

  programs.spicetify = with spicePkgs; {
    enable = true;
    # theme = themes."Comfy";

    theme = themes.text;
    # colorScheme = "ylx-ui";

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

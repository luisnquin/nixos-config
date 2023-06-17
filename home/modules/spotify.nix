{
  spicetify-nix,
  config,
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
    theme = themes.Comfy;
    # theme = themes.Ziro;
    # colorScheme = "ylx-ui"; # Kebab-case

    enabledExtensions = with extensions; [
      historyShortcut
      fullAppDisplay
      fullAlbumDate
      hidePodcasts
      popupLyrics
      songStats
      trashbin
      shuffle
      wikify
      genre
    ];
  };
}

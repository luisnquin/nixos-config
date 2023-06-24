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
    theme = themes."catppuccin-mocha";
    # colorScheme = "catppuccin-macchiato";
    # theme = themes.Ziro;
    # colorScheme = "ylx-ui"; # Kebab-case

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

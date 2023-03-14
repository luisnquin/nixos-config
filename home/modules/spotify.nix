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
    theme = themes.Ziro;
    colorScheme = "red-light"; # Kebab-case

    enabledExtensions = with extensions; [
      historyShortcut
      fullAppDisplay
      fullAlbumDate
      popupLyrics
      trashbin
      shuffle
      genre
    ];
  };
}

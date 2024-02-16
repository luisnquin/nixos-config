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

  # services.spotifyd = {
  #   enable = true;
  #   package = pkgs.spotifyd;

  #   settings = {
  #     global = {
  #       username = libx.base64.decode "eWVzZWxvbnk="; # :)
  #       password = libx.base64.decode "X1dob0lzQWZyYWlkT2ZDaGFuZ2U5OTg=";
  #       device_name = host.name;
  #     };
  #   };
  # };
}

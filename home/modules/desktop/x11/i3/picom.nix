{pkgs, ...}: {
  # https://mipmip.github.io/home-manager-option-search/?query=services.picom.
  services.picom = {
    enable = true;
    package = pkgs.picom-next;
    backend = "xrender";

    fade = true;
    fadeDelta = 10;
    shadow = false;

    activeOpacity = 0.9;
    menuOpacity = 0.8;

    settings = {
      corner-radius = 10;
      detect-rounded-corners = false;
      # https://github.com/dunst-project/dunst/issues/697
      unredir-if-possible = true;
    };
  };
}

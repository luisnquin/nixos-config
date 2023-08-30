{pkgs, ...}: {
  # https://mipmip.github.io/home-manager-option-search/?query=services.picom.
  services.picom = {
    enable = true;
    package = pkgs.picom-next;
    activeOpacity = 0.9;
    backend = "xrender";
    fade = true;
    fadeDelta = 10;
    shadow = false;

    settings = {
      corner-radius = 1;
      detect-rounded-corners = false;
    };

    # vSync = false;
  };
}

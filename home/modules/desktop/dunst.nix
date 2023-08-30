{pkgs, ...}: {
  services.dunst = {
    enable = true;
    package = pkgs.dunst;

    iconTheme = with pkgs; {
      name = "Adwaita";
      package = gnome.adwaita-icon-theme;
      size = "16x16";
    };

    settings = {
      global = {
        width = 300;
        height = 300;
        offset = "30x50";
        origin = "top-right";
        transparency = 10;
        frame_color = "#eceff1";
        font = "Cascadia Code 9";
      };

      urgency_normal = {
        background = "#37474f";
        foreground = "#eceff1";
        timeout = 10;
      };
    };
  };
}

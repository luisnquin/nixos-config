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
        font = "Cascadia Code 9";
        frame_color = "#8d31de";
        width = 320;
        height = 300;
        padding = 13;
        offset = "30x40";
        origin = "top-right";
        transparency = 10; # X11 only
        corner_radius = 3;
        notification_limit = 20;
        scale = 2;
      };

      urgency_low = {
        background = "#262121";
        foreground = "#888888";
        timeout = 10;
      };

      urgency_normal = {
        background = "#371f4d";
        foreground = "#eceff1";
        timeout = 10;
      };

      urgency_critical = {
        background = "#900000";
        foreground = "#ffffff";
        frame_color = "#ff0000";
        timeout = 20;
        override_pause_level = 20;
      };
    };
  };
}

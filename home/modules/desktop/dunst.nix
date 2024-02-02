{pkgs, ...}: {
  services.dunst = {
    enable = true;
    package = pkgs.dunst;

    iconTheme = with pkgs; {
      name = "Adwaita";
      package = gnome.adwaita-icon-theme;
      size = "16x16";
    };

    settings = let
      v2 = {
        global = {
          font = "Cascadia Code 9";
          frame_color = "#1e1b29";
          width = 330;
          height = 300;
          padding = 20;
          offset = "40x50";
          origin = "top-right";
          transparency = 10; # X11 only
          corner_radius = 3;
          notification_limit = 20;
          scale = 3;
        };

        urgency_low = {
          background = "#1b1925";
          foreground = "#cbcacf";
          timeout = 10;
        };

        urgency_normal = {
          background = "#1b1924de";
          foreground = "#cbcacf";
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
    in
      v2;
  };
}

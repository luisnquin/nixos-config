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
        height = 310;
        offset = "60x90";
        origin = "top-right";
        transparency = 8;
        frame_color = "#dae2e8";
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

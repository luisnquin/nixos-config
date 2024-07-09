{pkgs, ...}: {
  gtk = {
    enable = true;
    theme = {
      name = "WhiteSur";
      package = pkgs.whitesur-gtk-theme;
    };

    iconTheme = {
      name = "Adwaita";
      package = pkgs.adwaita-icon-theme;
    };
  };
}

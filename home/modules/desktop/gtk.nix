{pkgs, ...}: {
  gtk = with pkgs; {
    enable = true;
    theme = {
      name = "WhiteSur";
      package = whitesur-gtk-theme;
    };

    iconTheme = {
      name = "Adwaita";
      package = gnome.adwaita-icon-theme;
    };
  };
}

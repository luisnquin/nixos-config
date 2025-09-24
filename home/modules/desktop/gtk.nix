{pkgs, ...}: {
  gtk = {
    enable = true;
    theme = {
      name = "Vimix-light-doder";
      package = pkgs.vimix-gtk-themes;
    };

    iconTheme = {
      name = "Adwaita";
      package = pkgs.adwaita-icon-theme;
    };
  };
}

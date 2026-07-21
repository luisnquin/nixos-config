{pkgs, ...}: {
  # Global multiplier on GTK4 animation clocks (1.0 = default)
  home.sessionVariables.GTK_SLOWDOWN = "0.1";

  gtk = {
    enable = true;
    theme = {
      name = "Vimix-light-doder";
      package = pkgs.vimix-gtk-themes;
    };

    gtk4.theme = {
      name = "Vimix-light-doder";
      package = pkgs.vimix-gtk-themes;
    };

    iconTheme = {
      name = "Adwaita";
      package = pkgs.adwaita-icon-theme;
    };
  };
}

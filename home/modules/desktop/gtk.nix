{pkgs, ...}: {
  gtk = {
    enable = true;
    theme = {
      name = "Juno";
      package = pkgs.juno-theme;
    };

    iconTheme = {
      name = "Adwaita";
      package = pkgs.adwaita-icon-theme;
    };
  };
}

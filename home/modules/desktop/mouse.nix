{pkgs-extra, ...}: {
  home.pointerCursor = {
    gtk.enable = true;
    x11.enable = true;

    name = "Vimix-Cursors";
    size = 32;
    package = pkgs-extra.vimix-cursor-theme;
  };
}

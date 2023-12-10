{pkgsx, ...}: {
  home.pointerCursor = {
    gtk.enable = true;
    x11.enable = true;

    name = "Vimix-Cursors";
    size = 32;
    package = pkgsx.vimix-cursor-theme;
  };
}

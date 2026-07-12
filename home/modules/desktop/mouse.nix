{pkgs, ...}: {
  home.pointerCursor = {
    enable = true;
    gtk.enable = true;
    x11.enable = true;
    hyprcursor.enable = true;

    name = "Vimix-Cursors";
    size = 32;
    package = pkgs.vimix-cursor-theme;
  };
}

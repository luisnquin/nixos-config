{
  # https://github.com/escherdragon/zathura/blob/master/zathura.desktop
  xdg = {
    desktopEntries = {
      zathura = {
        name = "zathura";
        type = "Application";
        comment = "A minimalistic PDF viewer";
        exec = "zathura %f";
        terminal = false;
        categories = ["Office" "Viewer"];
        mimeType = ["application/pdf"];
      };
    };

    mimeApps.defaultApplications = {
      "application/pdf" = "zathura.desktop";
    };
  };

  programs.zathura = {
    enable = true;
    options = {
      window-title-page = true;
      window-height = 850;

      scroll-page-aware = true;
      scroll-full-overlap = "0.01";
      scroll-step = 100;

      zoom-min = 5;
      guioptions = "v";

      font = "inconsolata 15";

      default-bg = "#000000";
      default-fg = "#F7F7F6";

      statusbar-fg = "#B0B0B0";
      statusbar-bg = "#202020";

      notification-bg = "#90A959";
      notification-fg = "#151515";

      notification-error-bg = "#dedede";
      notification-error-fg = "#ac4142";

      notification-warning-bg = "#AC4142";
      notification-warning-fg = "#151515";

      highlight-color = "#F4BF75";
      highlight-active-color = "#6A9FB5";

      completion-bg = "#303030";
      completion-fg = "#E0E0E0";

      completion-highlight-fg = "#151515";
      completion-highlight-bg = "#90A959";

      render-loading = true;
    };
  };
}

{host, ...}: {
  xdg.mimeApps = let
    dolphinDesktop = "org.kde.dolphin.desktop";
    vivaldiDesktop = "vivaldi-stable.desktop";

    browserDesktop =
      # fail-fast, add other if needed
      {
        "brave" = "brave-browser.desktop";
        "vivaldi-stable" = vivaldiDesktop;
        "google-chrome" = "google-chrome.desktop";
      }
      ."${host.browser}";
  in {
    enable = true;

    associations.added = {
      "inode/directory" = dolphinDesktop;
      "x-scheme-handler/geo" = "wheelmap-geo-handler.desktop";
      "x-scheme-handler/http" = browserDesktop;
      "x-scheme-handler/https" = browserDesktop;
      "x-scheme-handler/mailto" = vivaldiDesktop;
    };

    defaultApplications = {
      "x-scheme-handler/mailto" = vivaldiDesktop;
      "x-scheme-handler/slack" = "slack.desktop";
      "inode/directory" = dolphinDesktop;

      "x-scheme-handler/unknown" = browserDesktop;
      "x-scheme-handler/https" = browserDesktop;
      "x-scheme-handler/about" = browserDesktop;
      "x-scheme-handler/http" = browserDesktop;
      "text/html" = browserDesktop;
    };
  };
}

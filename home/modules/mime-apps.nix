{host, ...}: {
  xdg.mimeApps = let
    dolphinDesktop = "org.kde.dolphin.desktop";
    vivaldiDesktop = "vivaldi-stable.desktop";
    braveDesktop = "brave.desktop";
  in {
    enable = true;

    associations.added = {
      "inode/directory" = dolphinDesktop;
      "x-scheme-handler/geo" = "wheelmap-geo-handler.desktop";
      "x-scheme-handler/http" = braveDesktop;
      "x-scheme-handler/https" = braveDesktop;
      "x-scheme-handler/mailto" = vivaldiDesktop;
    };

    defaultApplications =
      {
        "x-scheme-handler/mailto" = vivaldiDesktop;
        "x-scheme-handler/slack" = "slack.desktop";
        "inode/directory" = dolphinDesktop;
      }
      // (
        if host.browser == "brave"
        then {
          "x-scheme-handler/unknown" = braveDesktop;
          "x-scheme-handler/https" = braveDesktop;
          "x-scheme-handler/about" = braveDesktop;
          "x-scheme-handler/http" = braveDesktop;
          "text/html" = braveDesktop;
        }
        else {}
      );
  };
}

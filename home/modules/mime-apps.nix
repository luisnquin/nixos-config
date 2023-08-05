{host, ...}: {
  xdg.mimeApps = {
    enable = true;

    defaultApplications =
      {
        "x-scheme-handler/mailto" = "vivaldi-stable.desktop";
        "x-scheme-handler/slack" = "slack.desktop";
        "inode/directory" = "org.kde.dolphin.desktop";
      }
      // (
        if host.browser == "brave"
        then let
          braveDesktop = "brave.desktop";
        in {
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

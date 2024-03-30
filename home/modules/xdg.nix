{
  config,
  host,
  ...
}: {
  xdg = {
    enable = true;
    userDirs = let
      home = config.home.homeDirectory;
    in {
      enable = true;
      createDirectories = true;
      desktop = null;
      download = "${home}/Downloads";
      documents = "${home}/Documents";
      pictures = "${home}/Pictures";
      music = null;
      publicShare = null;
      templates = null;
      videos = null;
      extraConfig = {
        XDG_PROJECTS_DIR = "${home}/Projects";
        XDG_WORK_DIR = "${home}/Work";
      };
    };

    mimeApps = let
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
  };
}

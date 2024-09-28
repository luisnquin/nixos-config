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
      templates = "${home}/Experiments";
      videos = "${home}/Videos";
      extraConfig = {
        XDG_PROJECTS_DIR = "${home}/Projects";
        XDG_WORK_DIR = "${home}/Work";
      };
    };

    mimeApps = let
      dolphin = ["org.kde.dolphin.desktop"];
      vlc = ["vlc.desktop"];
      zathura = ["zathura.desktop"];
      vscode = ["code.desktop"];

      browser =
        # fail-fast, add other if needed
        [
          {
            "brave" = "brave-browser.desktop";
            "google-chrome" = "google-chrome.desktop";
          }
          ."${host.browser}"
        ];

      associations = {
        "inode/directory" = dolphin;
        "x-scheme-handler/geo" = "wheelmap-geo-handler.desktop";
        "x-scheme-handler/http" = browser;
        "x-scheme-handler/https" = browser;
        "x-scheme-handler/mailto" = browser;
        "x-scheme-handler/slack" = "slack.desktop";

        "x-scheme-handler/unknown" = browser;
        "x-scheme-handler/about" = browser;

        "application/pdf" = zathura;
        "application/json" = browser;

        "image/*" = browser; # feh.desktop is not working :((
        "video/*" = vlc; # not working as expected...
        "video/x-matroska" = vlc;
        "video/quicktime" = vlc;

        "audio/*" = vlc;

        "text/x-python" = vscode;
        "text/plain" = browser;
        "text/html" = browser;
      };
    in {
      enable = true;
      associations.added = associations;
      defaultApplications = associations;
    };
  };
}

{config, ...}: {
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

      associations = {
        "inode/directory" = dolphin;
        "x-scheme-handler/geo" = "wheelmap-geo-handler.desktop";
        "x-scheme-handler/slack" = "slack.desktop";

        "application/pdf" = zathura;

        "video/*" = vlc; # not working as expected...
        "video/x-matroska" = vlc;
        "video/quicktime" = vlc;

        "audio/*" = vlc;

        "text/x-python" = vscode;
      };
    in {
      enable = true;
      associations.added = associations;
      defaultApplications = associations;
    };
  };
}

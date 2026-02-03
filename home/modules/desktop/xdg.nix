{config, ...}: {
  # let me be the lesser of a beautiful man
  xdg.configFile."mimeapps.list".force = true;

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
      videos = "${home}/Videos";
      extraConfig = {
        XDG_PROJECTS_DIR = "${home}/Projects";
        # XDG_WORK_DIR = "${home}/Work";
      };
    };

    mimeApps = let
      dolphin = ["org.kde.dolphin.desktop"];
      vlc = ["vlc.desktop"];
      zathura = ["zathura.desktop"];
      vscode = ["code.desktop"];

      associations = {
        "inode/directory" = dolphin;
        "application/pdf" = zathura;

        "video/mp4" = vlc;
        "video/x-matroska" = vlc;
        "video/webm" = vlc;
        "video/x-msvideo" = vlc;

        "audio/mpeg" = vlc;
        "audio/flac" = vlc;
        "audio/ogg" = vlc;

        "text/x-python" = vscode;
      };
    in {
      enable = true;
      associations.added = associations;
      defaultApplications = associations;
    };
  };
}

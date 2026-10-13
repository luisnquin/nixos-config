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
        # WORK = "${home}/Work";
      };
    };

    mimeApps = let
      nautilus = ["org.gnome.Nautilus.desktop"];
      vlc = ["vlc.desktop"];
      zathura = ["zathura.desktop"];
      zed = ["dev.zed.Zed.desktop"];
      claude = ["claude-desktop.desktop"];

      associations = {
        "inode/directory" = nautilus;
        "application/pdf" = zathura;
        "x-scheme-handler/claude" = claude;

        "video/mp4" = vlc;
        "video/x-matroska" = vlc;
        "video/webm" = vlc;
        "video/x-msvideo" = vlc;

        "audio/mpeg" = vlc;
        "audio/flac" = vlc;
        "audio/ogg" = vlc;

        "text/x-python" = zed;
        "text/plain" = zed;
        "application/json" = zed;
      };
    in {
      enable = true;
      associations.added = associations;
      defaultApplications = associations;
    };
  };
}

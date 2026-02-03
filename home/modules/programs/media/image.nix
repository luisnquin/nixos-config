{pkgs, ...}: let
  mkWithDesktopItemWrapper = {
    package,
    desktopName,
    exec,
    genericName,
    name ? desktopName,
  }: {
    new = package.overrideAttrs (old: {
      nativeBuildInputs = (old.nativeBuildInputs or []) ++ [pkgs.copyDesktopItems];
      desktopItems = [
        (pkgs.makeDesktopItem {
          inherit name desktopName exec genericName;
        })
      ];
    });
    desktopFile = "${desktopName}.desktop";
  };

  feh = mkWithDesktopItemWrapper {
    package = pkgs.feh;
    desktopName = "feh";
    exec = "feh --bg-scale %f";
    genericName = "Image Viewer";
  };

  sxiv = mkWithDesktopItemWrapper {
    package = pkgs.sxiv;
    desktopName = "svix";
    name = "sxiv (gif)";
    exec = "sxiv -a %f";
    genericName = "Gif Viewer";
  };
in {
  home.packages = [
    pkgs.imagemagick
    sxiv.new
    feh.new
  ];

  xdg.mimeApps = let
    chromiumDesktop = "chromium-browser.desktop";

    defaultApps = {
      "image/svg+xml" = [feh.desktopFile];
      "image/png" = [feh.desktopFile];
      "image/jpg" = [feh.desktopFile];
      "image/jpeg" = [feh.desktopFile];
      "image/gif" = [sxiv.desktopFile];
    };

    unsetChromium = builtins.mapAttrs (_: _: [chromiumDesktop]) defaultApps;
  in {
    defaultApplications = defaultApps;

    associations = {
      added = defaultApps;
      removed = unsetChromium;
    };
  };
}

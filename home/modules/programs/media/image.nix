{pkgs, ...}: let
  mkWithDesktopItemWrapper = package: name: desktopName: exec: genericName: {
    new = package.overrideAttrs (old: {
      nativeBuildInputs = (old.nativeBuildInputs or []) ++ [pkgs.copyDesktopItems];

      desktopItems = [
        (pkgs.makeDesktopItem {
          inherit name desktopName exec genericName;
        })
      ];
    });
    desktopFileName = desktopName;
  };

  feh = mkWithDesktopItemWrapper pkgs.feh "feh" "feh" "feh --bg-scale %f" "Image Viewer";
  sxiv = mkWithDesktopItemWrapper pkgs.sxiv "sxiv (gif)" "svix" "sxiv -a %f" "Gif Viewer";
in {
  home.packages = [
    pkgs.imagemagick
    sxiv.new
    feh.new
  ];

  xdg.mimeApps = let
    associations =
      builtins.listToAttrs (map (mimeType: {
          name = mimeType;
          value = feh.desktopFileName;
        })
        [
          "image/svg+xml"
          "image/png"
          "image/jpg"
          "image/jpeg"
        ])
      // {
        name = "image/gif";
        value = sxiv.desktopFileName;
      };
  in {
    defaultApplications = associations;

    associations = {
      added = associations;
      removed = {
        "image/gif" = "chromium-browser.desktop";
        "image/jpg" = "chromium-browser.desktop";
        "image/jpeg" = "chromium-browser.desktop";
        "image/png" = "chromium-browser.desktop";
        "image/svg+xml" = "chromium-browser.desktop";
      };
    };
  };
}

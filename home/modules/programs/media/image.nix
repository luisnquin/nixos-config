{pkgs, ...}: let
  feh = pkgs.feh.overrideAttrs (old: {
    nativeBuildInputs = old.nativeBuildInputs ++ [pkgs.copyDesktopItems];

    desktopItems = [
      (pkgs.makeDesktopItem {
        name = "feh";
        exec = "feh --bg-scale %f";
        icon = "feh";
        desktopName = "feh";
        genericName = "Image Viewer";
      })
    ];
  });

  sxiv = pkgs.sxiv.overrideAttrs (old: {
    nativeBuildInputs = [pkgs.copyDesktopItems];

    desktopItems = [
      (pkgs.makeDesktopItem {
        name = "sxiv (gif)";
        exec = "sxiv -a";
        desktopName = "sxiv";
        genericName = "Gif Viewer";
      })
    ];
  });
in {
  home.packages = [
    pkgs.imagemagick
    feh
    sxiv
  ];

  xdg.mimeApps = let
    associations =
      builtins.listToAttrs (map (mimeType: {
          name = mimeType;
          value = "feh.desktop";
        })
        [
          "image/svg+xml"
          "image/png"
          "image/jpeg"
        ])
      // {
        name = "image/gif";
        value = "sxiv.desktop";
      };
  in {
    associations.added = associations;
    defaultApplications = associations;
  };
}

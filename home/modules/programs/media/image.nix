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
in {
  home.packages = [
    feh
  ];

  xdg.mimeApps = let
    associations = builtins.listToAttrs (map (mimeType: {
        name = mimeType;
        value = "feh.desktop";
      })
      [
        "image/svg+xml"
        "image/png"
        "image/jpeg"
      ]);
  in {
    associations.added = associations;
    defaultApplications = associations;
  };
}

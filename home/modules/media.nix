{pkgs, ...}: {
  home.packages = let
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
  in [
    pkgs.vlc
    feh
  ];
}

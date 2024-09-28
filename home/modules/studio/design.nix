{pkgs, ...}: {
  home.packages = let
    inherit (pkgs) lib;
  in [
    (
      pkgs.figma-linux.overrideAttrs
      (old: {
        nativeBuildInputs = old.nativeBuildInputs ++ [pkgs.copyDesktopItems];
        desktopItems = [
          (
            pkgs.makeDesktopItem {
              name = "Figma (linux)";
              exec = lib.getExe pkgs.figma-linux;
              icon = "${placeholder "out"}/lib/figma-linux.png";
              desktopName = "Figma (linux)";
              genericName = old.meta.description;
            }
          )
        ];
      })
    )
  ];
}

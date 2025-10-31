{pkgs, ...}: {
  home.packages = with pkgs; [
    blender
    orca-slicer
    freecad-wayland
    inkscape # vector graphics, before 3d

    (fstl.overrideAttrs (
      old: {
        nativeBuildInputs = old.nativeBuildInputs ++ [pkgs.copyDesktopItems];
        desktopItems = [
          (pkgs.makeDesktopItem {
            name = "fstl";
            comment = "A fast STL file viewer";
            exec = "${lib.getExe pkgs.fstl}";
            desktopName = "FSTL";
            keywords = ["3d"];
          })
        ];
      }
    ))
  ];

  services.flatpak.packages = [
    {
      appId = "com.bambulab.BambuStudio";
      origin = "flathub";
    }
  ];
}

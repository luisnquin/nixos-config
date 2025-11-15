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

  xdg.mimeApps = let
    associationsFor = value:
      builtins.listToAttrs (map (name: {
        inherit name value;
      }) ["model/stl" "application/sla"]);
  in {
    associations = {
      added = associationsFor "fstl.desktop";
      removed = associationsFor "OrcaSlicer.desktop" // associationsFor "com.bambulab.BambuStudio.desktop";
    };
    defaultApplications = associationsFor "fstl.desktop";
  };

  services.flatpak.packages = [
    {
      appId = "com.bambulab.BambuStudio";
      origin = "flathub";
    }
  ];
}

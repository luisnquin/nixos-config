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
    associations = builtins.listToAttrs (map (name: {
      inherit name;
      value = "fstl.desktop";
    }) ["model/stl" "application/sla"]);
  in {
    associations.added = associations;
    defaultApplications = associations;
  };

  services.flatpak.packages = [
    {
      appId = "com.bambulab.BambuStudio";
      origin = "flathub";
    }
  ];
}

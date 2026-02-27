{
  pkgs,
  lib,
  inputs,
  ...
}: let
  _3mf2stl = inputs."3mf2stl".packages.${pkgs.system}.default;

  convertScript = pkgs.writeShellScriptBin "3mf2stl-convert" ''
    #!/bin/bash
    input_file="$1"
    if [[ "$input_file" == *.3mf ]]; then
      output_file="''${input_file%.3mf}.stl"
      ${lib.getExe _3mf2stl} "$input_file" "$output_file"
      ${pkgs.xdg-utils}/bin/xdg-open "$output_file"
    fi
  '';
in {
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
    _3mf2stl
  ];

  xdg.desktopEntries."3mf2stl" = {
    name = "3mf2stl Converter";
    exec = "${convertScript}/bin/3mf2stl-convert %f";
    mimeType = ["model/3mf"];
    categories = ["Graphics"];
    comment = "Convert 3MF to STL and open";
  };

  xdg.mimeApps = let
    associationsFor = value:
      builtins.listToAttrs (map (name: {
        inherit name value;
      }) ["model/stl" "application/sla"]);
  in {
    associations = {
      added = associationsFor "fstl.desktop" // {"model/3mf" = "3mf2stl.desktop";};
      removed = associationsFor "OrcaSlicer.desktop" // associationsFor "com.bambulab.BambuStudio.desktop";
    };
    defaultApplications = associationsFor "fstl.desktop" // {"model/3mf" = "3mf2stl.desktop";};
  };

  services.flatpak = {
    packages = [
      {
        appId = "com.bambulab.BambuStudio";
        origin = "flathub";
      }
    ];

    overrides."com.bambulab.BambuStudio".Context = {
      filesystems = ["xdg-download:rw"];
    };
  };
}

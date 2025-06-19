{pkgs, ...}: {
  home.packages = [
    pkgs.blender
    pkgs.bambu-studio
    pkgs.orca-slicer
  ];
}

{pkgs, ...}: {
  home.packages = [
    pkgs.blender
    pkgs.orca-slicer
  ];
}

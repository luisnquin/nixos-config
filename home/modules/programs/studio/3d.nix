{pkgs, ...}: {
  home.packages = with pkgs; [
    blender
    orca-slicer
    freecad-wayland
  ];
}

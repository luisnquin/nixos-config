{pkgs, ...}: {
  home.packages = with pkgs; [
    blender
    orca-slicer
    freecad-wayland
    inkscape # vector graphics, before 3d
  ];

  services.flatpak.packages = [
    {
      appId = "com.bambulab.BambuStudio";
      origin = "flathub";
    }
  ];
}

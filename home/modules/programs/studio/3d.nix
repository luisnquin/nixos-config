{pkgs, ...}: {
  home.packages = with pkgs; [
    blender
    orca-slicer
    freecad-wayland
  ];

  services.flatpak.packages = [
    {
      appId = "com.bambulab.BambuStudio";
      origin = "flathub";
    }
  ];
}

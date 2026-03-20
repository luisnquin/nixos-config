{pkgs, ...}: {
  home.packages = with pkgs.scripts; [
    sys-brightness
    sys-sound
  ];
}

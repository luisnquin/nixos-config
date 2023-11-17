{pkgs, ...}: {
  home.packages = with pkgs; [
    gnome.nautilus
    ranger
  ];
}

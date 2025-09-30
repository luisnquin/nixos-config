{pkgs, ...}: {
  home.packages = with pkgs; [
    pkgs.ranger
    nautilus
  ];
}

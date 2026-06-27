{pkgs, ...}: {
  programs.cliphizt.enable = true;

  home.packages = [
    pkgs.cliplenz
  ];
}

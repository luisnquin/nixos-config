{pkgs, ...}: {
  home.packages = [
    pkgs.git-cliff
    pkgs.git-open
  ];
}

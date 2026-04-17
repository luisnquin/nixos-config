{pkgs, ...}: {
  home.packages = [
    # https://github.com/ifd3f/caligula
    pkgs.caligula # TUI for imaging disks
  ];
}

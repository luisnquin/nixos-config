{pkgs, ...}: {
  home.packages = with pkgs; [
    # jetbrains.datagrip
    sqlite-web
    pgweb
  ];
}

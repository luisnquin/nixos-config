{
  pkgs,
  pkgsx,
  ...
}: {
  home.packages = with pkgs; [
    # jetbrains.datagrip
    pkgsx.dbeaver
    sqlite-web
    pgweb
  ];
}

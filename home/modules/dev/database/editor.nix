{
  pkgs,
  pkgsx,
  ...
}: {
  home.packages = with pkgs; [
    pkgsx.dbeaver
    sqlite-web
    pgweb
  ];
}

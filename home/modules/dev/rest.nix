{
  pkgsx,
  pkgs,
  ...
}: {
  home.packages = [
    pkgsx.openapi-tui
    pkgs.insomnia
    pkgsx.yaak # REST client
    pkgsx.atac
  ];
}

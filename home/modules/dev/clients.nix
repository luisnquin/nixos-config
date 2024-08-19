{
  pkgsx,
  pkgs,
  ...
}: {
  home.packages = [
    # pkgsx.openapi-tui
    pkgs.insomnia # another REST client
    pkgsx.yaak # REST client
    # pkgsx.atac
    pkgs.websocat
  ];
}

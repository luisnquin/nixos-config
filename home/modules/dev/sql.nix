{
  pkgsx,
  pkgs,
  ...
}: {
  home.packages = with pkgs; [
    pkgsx.pg-ping
    pgweb
    sqlc # Go code generator
  ];
}

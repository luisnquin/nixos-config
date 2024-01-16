{
  pkgsx,
  pkgs,
  ...
}: {
  home.packages = with pkgs; [
    pkgsx.pg-ping
    sqlite-web # SQLite
    pgweb # Postgres
    sqlc # Go code generator
  ];
}

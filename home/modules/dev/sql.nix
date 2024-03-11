{
  pkgsx,
  pkgs,
  ...
}: {
  home.packages = with pkgs; [
    pkgsx.pg-ping
    pkgsx.dbeaver

    # Builder
    sqlc
    sqlite-web # SQLite
    pgweb # Postgres
  ];
}

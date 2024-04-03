{
  pkgsx,
  pkgs,
  ...
}: {
  home.packages = with pkgs; [
    pkgsx.pg-ping
    pkgsx.dbeaver
    # pkgsx.lazysql

    # Builder
    sqlc
    sqlite-web # SQLite
    pgweb # Postgres
  ];
}

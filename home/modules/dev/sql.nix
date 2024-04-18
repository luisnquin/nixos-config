{
  pkgsx,
  pkgs,
  ...
}: {
  home.packages = with pkgs; [
    pkgsx.dbeaver
    # pkgsx.lazysql

    # Builder
    sqlc
    sqlite-web # SQLite
    pgweb # Postgres
  ];
}

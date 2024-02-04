{
  pkgsx,
  pkgs,
  ...
}: {
  home.packages = with pkgs; [
    pkgsx.pg-ping

    # Builder
    sqlc

    sqlite-web # SQLite
    dbeaver
    pgweb # Postgres
  ];
}

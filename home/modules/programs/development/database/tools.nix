{
  pkgs,
  lib,
  ...
}: let
  postgresql-client = (
    pkgs.linkFarm "postgresql-client" [
      {
        name = "bin/psql";
        path = lib.getExe' pkgs.postgresql_19 "psql";
      }
    ]
  );

  redis-cli = (
    pkgs.linkFarm "redis-cli" [
      {
        name = "bin/redis-cli";
        path = lib.getExe' pkgs.redis "redis-cli";
      }
    ]
  );
in {
  home.packages = with pkgs; [
    postgresql-client # for server stuff
    redis-cli
    sqlite
  ];
}

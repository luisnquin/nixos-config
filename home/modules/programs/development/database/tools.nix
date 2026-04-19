{pkgs, ...}: let
  postgresql-client = (
    pkgs.linkFarm "postgresql-client" [
      {
        name = "bin/psql";
        path = "${pkgs.postgresql}/bin/psql";
      }
    ]
  );

  redis-cli = (
    pkgs.linkFarm "redis-cli" [
      {
        name = "bin/redis-cli";
        path = "${pkgs.redis}/bin/redis-cli";
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

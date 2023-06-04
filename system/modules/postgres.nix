{
  config,
  pkgs,
  ...
}: {
  services.postgresql = {
    # More advanced options here: https://nixos.wiki/wiki/PostgreSQL
    enable = true;
    package = pkgs.postgresql_15;
    port = 5432;
    settings = {
      log_statement = "all";
      logging_collector = true;
      log_connections = true;
      log_disconnections = true;
    };
  };
}

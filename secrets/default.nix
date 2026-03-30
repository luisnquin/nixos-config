{
  system,
  agenix,
}: {
  age.secrets = {
    "certs/ccd/rootCA.crt" = {
      file = ./certs/ccd/rootCA.crt.age;
      owner = "root";
      group = "root";
      mode = "0644";
    };
    "certs/ccd/rootCA.key" = {
      file = ./certs/ccd/rootCA.key.age;
      owner = "root";
      group = "root";
      mode = "0600";
    };
    "certs/ccd/wildcard.crt" = {
      file = ./certs/ccd/wildcard.crt.age;
      owner = "root";
      group = "root";
      mode = "0644";
    };
    "certs/ccd/wildcard.key" = {
      file = ./certs/ccd/wildcard.key.age;
      owner = "caddy";
      group = "caddy";
      mode = "0600";
    };
  };

  environment.systemPackages = [agenix.packages.${system}.default];
}

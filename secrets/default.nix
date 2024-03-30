{
  system,
  agenix,
}: {
  age.secrets = {
    acme-env-luisquinones-me = {
      file = ./acme-env-luisquinones-me.age;
      mode = "770";
      owner = "nginx";
      group = "nginx";
    };
  };

  environment.systemPackages = [agenix.packages.${system}.default];
}

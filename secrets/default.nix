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

    spotify-access-secret = {
      file = ./spotify-access-secret.age;
    };
  };

  environment.systemPackages = [agenix.packages.${system}.default];
}

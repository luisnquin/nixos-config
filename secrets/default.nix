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

    acme-env-neticshard-com = {
      file = ./acme-env-neticshard-com.age;
    };

    spotify-access-secret = {
      file = ./spotify-access-secret.age;
    };
  };

  environment.systemPackages = [agenix.packages.${system}.default];
}

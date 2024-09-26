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

    "40a98b0e-fca3-42a4-8867-c453a33d45e6" = {
      file = ./40a98b0e-fca3-42a4-8867-c453a33d45e6.age;
      owner = "cloudflared";
      group = "cloudflared";
    };
  };

  environment.systemPackages = [agenix.packages.${system}.default];
}

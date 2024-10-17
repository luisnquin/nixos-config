{
  system,
  agenix,
}: {
  age.secrets = {
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

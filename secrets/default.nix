{
  system,
  agenix,
}: {
  age.secrets = {
    spotify-access-secret = {
      file = ./spotify-access-secret.age;
    };
  };

  environment.systemPackages = [agenix.packages.${system}.default];
}

{
  config,
  pkgs,
  ...
}: {
  environment.systemPackages = [
    pkgs.cloudflared
  ];

  services.cloudflared = {
    enable = true;

    # This doesn't seems to be reproducible at the current state.
    # A new tunnel and configurations will need to be issued by cloudflared's CLI.
    tunnels = {
      "40a98b0e-fca3-42a4-8867-c453a33d45e6" = {
        default = "http://localhost:4000";
        credentialsFile = config.age.secrets."40a98b0e-fca3-42a4-8867-c453a33d45e6".path;
      };
    };
  };
}

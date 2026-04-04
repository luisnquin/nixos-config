{config, ...}: {
  services.tailscale = {
    enable = true;
    authKeyFile = config.age.secrets."tailscale/auth-key".path;
  };
}

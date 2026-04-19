{config, ...}: {
  networking.wireguard.enable = true;

  services = {
    tailscale = {
      enable = true;
      authKeyFile = config.age.secrets."tailscale/auth-key".path;
    };

    mullvad-vpn.enable = true;
  };
}

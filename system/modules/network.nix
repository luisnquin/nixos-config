{
  networking = {
    firewall = {
      enable = true;
      allowedTCPPorts = [20 80 443 8088];
      allowPing = false;
    };

    wireguard.enable = true;
  };
}

{
  networking = {
    firewall = {
      enable = true;
      allowedTCPPorts = [357 5900];
      interfaces = {
        "wlo1".allowedTCPPorts = [
          357
          5900
        ];
      };
    };
  };
}

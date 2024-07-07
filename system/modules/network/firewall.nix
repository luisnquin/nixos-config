{
  networking = {
    firewall = {
      enable = true;
      allowedTCPPorts = [5900];
      interfaces = {
        "wlo1".allowedTCPPorts = [
          5900
        ];
      };
    };
  };
}

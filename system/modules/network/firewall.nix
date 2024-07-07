{
  networking = {
    firewall = let
      ports = [5900];
    in {
      enable = true;
      allowedTCPPorts = ports;
      interfaces."wlo1".allowedTCPPorts = ports;
    };
  };
}

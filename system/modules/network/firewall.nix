{
  networking = rec {
    nftables = {
      enable = true;
      ruleset = ''
        table ip nat {
          chain PREROUTING {
            type nat hook prerouting priority dstnat; policy accept;
            iifname "wlo1" tcp dport { 80, 443 } dnat to 10.100.0.3
          }
        }
      '';
    };

    firewall = {
      enable = true;
      allowedTCPPorts = [80 443];
    };

    nat = {
      enable = true;
      internalInterfaces = ["ens3"];
      externalInterface = "wlo1";
      forwardPorts =
        builtins.map (port: {
          sourcePort = port;
          proto = "tcp";
          destination = "10.100.0.3:${builtins.toString port}";
        })
        firewall.allowedTCPPorts;
    };
  };
}

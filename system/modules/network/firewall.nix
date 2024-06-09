{
  networking = {
    # nftables = {
    #   enable = true;
    #   ruleset = ''
    #     table ip nat {
    #       chain PREROUTING {
    #         type nat hook prerouting priority dstnat; policy accept;
    #         iifname "wlo1" tcp dport { 80, 443, 357 } dnat to 10.100.0.3
    #       }
    #     }
    #   '';
    # };

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

    #   nat = {
    #     enable = false;
    #     internalInterfaces = ["ens3"];
    #     externalInterface = "wlo1";
    #     forwardPorts =
    #       builtins.map (port: {
    #         sourcePort = port;
    #         proto = "tcp";
    #         destination = "10.100.0.3:${builtins.toString port}";
    #       })
    #       firewall.allowedTCPPorts;
    #   };
  };
}

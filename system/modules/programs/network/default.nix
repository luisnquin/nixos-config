{
  pkgs,
  host,
  ...
}: {
  imports = [
    ./caddy
    ./discovery.nix
    ./hosts.nix
    ./vpn.nix
    ./tools.nix
  ];

  networking = {
    interfaces."eno2".wakeOnLan.enable = true;

    networkmanager = {
      enable = true;
      unmanaged = [
        "interface-name:tailscale*"
        "interface-name:br-*"
        "interface-name:docker*"
        "interface-name:virbr*"
        "interface-name:vboxnet*"
        "interface-name:waydroid*"
        "type:bridge"
      ];
    };

    firewall = let
      ports = [5900 8081];
    in {
      enable = true;
      allowedTCPPorts = ports;
      interfaces."wlo1".allowedTCPPorts = ports;
    };

    hostName = host.name;
  };

  boot.kernel.sysctl = {
    "net.core.rmem_default" = 262144;
    "net.core.wmem_default" = 262144;
    "net.core.rmem_max" = 7500000;
    "net.core.wmem_max" = 7500000;
    "net.ipv4.udp_rmem_min" = 16384;
    "net.ipv4.udp_wmem_min" = 16384;
  };

  # most systems doesn't need this enabled
  systemd.services.NetworkManager-wait-online.enable = false;

  environment = {
    systemPackages = [
      pkgs.wakeonlan
    ];

    interactiveShellInit = builtins.readFile (builtins.path {
      name = "netutils.sh";
      path = ./netutils.sh;
    });
  };

  # BROKEN, BROKEN, BROKEN...
  # programs.wireshark.enable = true;
}

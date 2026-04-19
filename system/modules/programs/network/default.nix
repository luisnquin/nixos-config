{host, ...}: {
  imports = [
    ./caddy
    ./discovery.nix
    ./hosts.nix
    ./vpn.nix
    ./tools.nix
  ];

  networking = {
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

  # most systems doesn't need this enabled
  systemd.services.NetworkManager-wait-online.enable = false;

  environment.interactiveShellInit = builtins.readFile (builtins.path {
    name = "network-module-sh-script";
    path = ./shell.sh;
  });

  programs.wireshark.enable = true;
}

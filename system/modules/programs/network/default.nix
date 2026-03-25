{host, ...}: {
  imports = [
    ./caddy.nix
    ./discovery.nix
    ./hosts.nix
    ./tools.nix
  ];

  networking = {
    networkmanager.enable = true;
    firewall = let
      ports = [5900 8081];
    in {
      enable = true;
      allowedTCPPorts = ports;
      interfaces."wlo1".allowedTCPPorts = ports;
    };

    hostName = host.name;

    wireguard.enable = true;
  };

  # most systems doesn't need this enabled
  systemd.services.NetworkManager-wait-online.enable = false;

  environment.interactiveShellInit = builtins.readFile (builtins.path {
    name = "network-module-sh-script";
    path = ./shell.sh;
  });

  programs.wireshark.enable = true;

  services.mullvad-vpn.enable = true;
}

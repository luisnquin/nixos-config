{
  pkgs,
  host,
  ...
}: {
  imports = [
    ./hosts.nix
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

  environment = {
    systemPackages = with pkgs; [
      wirelesstools
      cloudflared
      wireshark
      inetutils
      iptables
      netcat
      nload
    ];

    interactiveShellInit = builtins.readFile (builtins.path {
      name = "network-module-sh-script";
      path = ./shell.sh;
    });
  };

  programs.wireshark.enable = true;

  services.mullvad-vpn.enable = true;
}

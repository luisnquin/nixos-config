{
  pkgs,
  host,
  ...
}: {
  imports = [
    ./hosts.nix
  ];

  networking = {
    networkmanager = {
      enable = true;
      unmanaged = ["wlp0s20f0u2"];
    };
    wlanInterfaces = {
      wlp0s20f0u2 = {
        device = "wlp0s20f0u2";
        type = "monitor";
        flags = "control";
      };
    };
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

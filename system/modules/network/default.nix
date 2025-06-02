{
  pkgs,
  host,
  ...
}: {
  imports = [
    ./cloudflare.nix
    ./firewall.nix
    ./hosts.nix
    ./nginx.nix
    ./vpn.nix
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

    hostName = host.name;

    wireguard.enable = true;
  };

  environment = {
    systemPackages = with pkgs; [
      wirelesstools
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
}

{
  pkgs,
  host,
  ...
}: {
  imports = [
    ./firewall.nix
    ./hosts.nix
    ./nginx.nix
    ./acme.nix
    ./vpn.nix
  ];

  networking = {
    networkmanager.enable = true;
    hostName = host.name;

    wireguard.enable = true;
  };

  environment = {
    systemPackages = [
      # pkgsx.netscanner
      pkgs.netcat
    ];

    interactiveShellInit = builtins.readFile (builtins.path {
      name = "network-module-sh-script";
      path = ./shell.sh;
    });
  };
}

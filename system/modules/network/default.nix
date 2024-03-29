{
  pkgsx,
  pkgs,
  host,
  ...
}: {
  imports = [
    ./services
    ./firewall.nix
  ];

  networking = {
    networkmanager.enable = true;
    hostName = host.name;

    wireguard.enable = true;
  };

  environment = {
    systemPackages = [pkgsx.netscanner pkgs.netcat];

    interactiveShellInit = builtins.readFile (builtins.path {
      name = "network-module-sh-script";
      path = ./shell.sh;
    });
  };
}

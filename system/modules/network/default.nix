{
  pkgsx,
  pkgs,
  host,
  ...
}: {
  networking = {
    networkmanager.enable = true;
    hostName = host.name;

    firewall = {
      enable = true;
      allowedTCPPorts = [20 80 443 8088];
      allowPing = true;
    };

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

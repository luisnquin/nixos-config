{
  config,
  pkgs,
  ...
}: {
  environment.systemPackages = [
    pkgs.cloudflared
  ];

  services.resolved = {
    enable = true;
    fallbackDns = builtins.map (name: "${name}#cloudflare-dns.com") config.networking.nameservers;
    dnsovertls = "true"; # not opportunistic
    domains = ["~."];
  };

  networking.nameservers = ["1.1.1.1" "1.0.0.1"];
}

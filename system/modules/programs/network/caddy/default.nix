{
  pkgs,
  config,
  ...
}: {
  imports = [
    ./options.nix
  ];

  security.pki.certificates = [
    config.age.secrets."certs/ccd/rootCA.crt".path
  ];

  environment.systemPackages = [pkgs.nss.tools];

  services.caddy = {
    enable = true;

    structuredVirtualHosts = let
      tls = {
        cert = config.age.secrets."certs/ccd/wildcard.crt".path;
        key = config.age.secrets."certs/ccd/wildcard.key".path;
      };
    in {
      "ccd.com.pe" = {
        inherit tls;
        reverseProxy.upstream = "localhost:5000";
      };

      "ccd.app" = {
        inherit tls;
        reverseProxy.upstream = "localhost:5100";
      };

      "magic.ccd.app" = {
        inherit tls;
        reverseProxy.upstream = "localhost:5200";
      };
    };
  };
}

{...}: {
  services.caddy = {
    enable = true;

    virtualHosts = {
      "magic.nshard.com".extraConfig = ''
        tls internal
        reverse_proxy localhost:7117
      '';
    };
  };
}

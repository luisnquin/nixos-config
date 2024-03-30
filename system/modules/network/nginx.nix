{
  config,
  pkgs,
  ...
}: {
  services.nginx = {
    enable = true;
    package = pkgs.nginxStable;

    defaultSSLListenPort = 443;
    enableReload = false;

    group = "nginx";
    user = "nginx";

    httpConfig = let
      rustypaste = {
        inherit (config.services.rustypaste.settings.server) address;
        domain = "localhost";
      };

      luisquinones-me = let
        domainsDir = "/var/lib/acme/luisquinones.me";
      in {
        crt = "${domainsDir}/cert.pem";
        key = "${domainsDir}/key.pem";
      };
    in ''
      log_format main
        '$remote_addr - $remote_user [$time_local] "$request" '
        '$status $body_bytes_sent "$http_referer" '
        '"$http_user_agent" "$http_x_forwarded_for"';

      client_max_body_size 10M;

      server {
          listen 80 default_server;
          server_name ${rustypaste.domain};
          return 301 https://$host$request_uri;
      }

      server {
          listen 443 ssl;
          server_name ${rustypaste.domain};

          ssl_certificate ${luisquinones-me.crt};
          ssl_certificate_key ${luisquinones-me.key};

          location / {
              proxy_pass http://${rustypaste.address};
              proxy_set_header Host $host;
              proxy_set_header X-Real-IP $remote_addr;
              proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
              proxy_set_header X-Forwarded-Proto $scheme;
          }
      }
    '';
  };
}

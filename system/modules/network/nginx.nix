{
  config,
  user,
  pkgs,
  lib,
  ...
}: let
  enableAll = false;
in {
  imports = [
    (import ./services {
      enable = enableAll;
      inherit user;
    })
  ];

  services.nginx = {
    enable = enableAll;
    package = pkgs.nginxStable;

    defaultSSLListenPort = 443;
    enableReload = false;

    group = "nginx";
    user = "nginx";

    httpConfig = let
      rustypaste = {
        inherit (config.services.rustypaste.settings.server) address;
        domain = "rp.localhost";
      };

      static-web-server = {
        address = let
          port = lib.strings.removePrefix "[::]:" config.services.static-web-server.listen;
        in "0.0.0.0:${port}";

        domain = "localhost";
      };

      localhost = let
        domainsDir = "${config.security.acme.certs."localhost.luisquinones.me".directory}";
      in {
        crt = "${domainsDir}/cert.pem";
        key = "${domainsDir}/key.pem";
      };

      luisquinones-me = let
        domainsDir = "${config.security.acme.certs."luisquinones.me".directory}";
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
          server_name ${static-web-server.domain};
          return 301 https://$host$request_uri;
      }

      server {
        listen 443 ssl;
        server_name ${static-web-server.domain};

        ssl_certificate ${localhost.crt};
        ssl_certificate_key ${localhost.key};

        location / {
            proxy_pass http://${static-web-server.address};
            proxy_set_header Host $host;
            proxy_set_header X-Real-IP $remote_addr;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto $scheme;
        }
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

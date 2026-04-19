{
  config,
  lib,
  ...
}:
with lib; let
  cfg = config.services.caddy.structuredVirtualHosts;

  mkVHost = _name: vhost: let
    tlsConfig =
      if vhost.tls == null
      then ""
      else if isString vhost.tls
      then "tls ${vhost.tls}"
      else "tls ${vhost.tls.cert} ${vhost.tls.key}";

    reverseProxyConfig =
      if vhost.reverseProxy == null
      then ""
      else "reverse_proxy ${vhost.reverseProxy.upstream} ${
        concatStringsSep " " vhost.reverseProxy.extraArgs
      }";
  in {
    serverAliases = vhost.serverAliases;

    extraConfig = ''
      ${optionalString (tlsConfig != "") tlsConfig}
      ${optionalString (reverseProxyConfig != "") reverseProxyConfig}
      ${vhost.extraConfig}
    '';
  };
in {
  options.services.caddy.structuredVirtualHosts = mkOption {
    type = types.attrsOf (types.submodule ({...}: {
      options = {
        serverAliases = mkOption {
          type = types.listOf types.str;
          default = [];
        };

        tls = mkOption {
          type = types.nullOr (types.oneOf [
            types.str
            (types.submodule {
              options = {
                cert = mkOption {type = types.path;};
                key = mkOption {type = types.path;};
              };
            })
          ]);
          default = null;
        };

        reverseProxy = mkOption {
          type = types.nullOr (types.submodule {
            options = {
              upstream = mkOption {
                type = types.str;
              };
              extraArgs = mkOption {
                type = types.listOf types.str;
                default = [];
              };
            };
          });
          default = null;
        };

        extraConfig = mkOption {
          type = types.lines;
          default = "";
        };
      };
    }));
    default = {};
  };

  config = mkIf (cfg != {}) {
    services.caddy = {
      enable = true;

      virtualHosts =
        mapAttrs mkVHost cfg;
    };
  };
}

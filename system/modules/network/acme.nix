{
  config,
  user,
  ...
}: {
  security.acme = {
    acceptTerms = true;
    defaults = {
      inherit (user) email;
      group = "nginx";
    };

    certs = {
      # https://shami.blog/2021/07/howto-letsencrypt-certificates-for-localhost/
      # localhost
      "localhost.luisquinones.me" = {
        dnsProvider = "cloudflare";
        credentialsFile = config.age.secrets.acme-env-luisquinones-me.path;
        extraDomainNames = ["*.localhost.luisquinones.me"];
      };

      "luisquinones.me" = {
        dnsProvider = "cloudflare";
        credentialsFile = config.age.secrets.acme-env-luisquinones-me.path;
      };
    };
  };
}

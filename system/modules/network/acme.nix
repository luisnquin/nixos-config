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
      "luisquinones.me" = {
        dnsProvider = "cloudflare";
        credentialsFile = config.age.secrets.acme-env-luisquinones-me.path;
      };
    };
  };
}

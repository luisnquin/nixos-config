{user, ...}: {
  security.acme = {
    acceptTerms = true;
    defaults = {
      inherit (user) email;
      group = "nginx";
    };
  };
}

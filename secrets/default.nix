{
  pkgs,
  user,
  ...
}: {
  sops = {
    defaultSopsFile = ./secrets.yaml;
    age.sshKeyPaths = ["/home/${user.alias}/.ssh/id_ed25519"];

    secrets = {
      "certs/ccd/rootCA.crt" = {
        owner = "root";
        group = "root";
        mode = "0644";
      };
      "certs/ccd/rootCA.key" = {
        owner = "root";
        group = "root";
        mode = "0600";
      };
      "certs/ccd/wildcard.crt" = {
        owner = "root";
        group = "root";
        mode = "0644";
      };
      "certs/ccd/wildcard.key" = {
        owner = "caddy";
        group = "caddy";
        mode = "0600";
      };
      "tailscale/auth-key" = {
        owner = "root";
        group = "root";
        mode = "0600";
      };
      "users/${user.alias}-password-hash".neededForUsers = true;
    };
  };

  environment.systemPackages = [pkgs.sops];
}

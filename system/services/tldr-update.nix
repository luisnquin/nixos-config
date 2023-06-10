{
  config,
  pkgs,
  ...
}: let
  username = (import ../../owner.nix).username;
in {
  systemd.services.tldr-update = {
    enable = true;
    description = "Updates tldr system pages";
    serviceConfig = {
      Type = "oneshot";
      ExecStart = ''${pkgs.sudo}/bin/sudo -u ${username} ${pkgs.tldr}/bin/tldr --update'';
    };

    wantedBy = ["multi-user.target"];
    after = ["successful-ping-to-google.service"];
  };
}

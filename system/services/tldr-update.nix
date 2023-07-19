{
  user,
  pkgs,
  ...
}: {
  systemd.services.tldr-update = {
    enable = true;
    description = "Updates tldr system pages";
    serviceConfig = {
      Type = "oneshot";
      ExecStart = ''${pkgs.sudo}/bin/sudo -u ${user.alias} ${pkgs.tldr}/bin/tldr --update'';
    };

    wantedBy = ["multi-user.target"];
    after = ["successful-ping-to-google.service"];
  };
}

{
  config,
  pkgs,
  ...
}: {
  systemd.services.tldrUpdate = {
    enable = true;
    description = "Updates tldr system pages";
    serviceConfig = {
      Type = "oneshot";
      ExecStart = ''${pkgs.bash}/bin/bash -c "${pkgs.tldr}/bin/tldr --update"'';
    };

    wantedBy = ["multi-user.target"];
    after = ["successful-ping-to-google.service"];
  };
}

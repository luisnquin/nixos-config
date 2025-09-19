{
  pkgs,
  user,
  ...
}: {
  environment.systemPackages = [pkgs.systemctl-tui];

  security.sudo.extraRules = [
    {
      users = [user.alias];
      commands = let
        commands = ["systemctl" "journalctl"];
      in
        builtins.map (cmd: {
          command = "/run/current-system/sw/bin/${cmd}";
          options = ["NOPASSWD"];
        })
        commands;
    }
  ];

  systemd.settings.Manager = {
    DefaultTimeoutStopSec = "15s";
  };

  services.journald.extraConfig = ''
    SystemMaxUse=1G
    MaxRetentionSec=7days
  '';
}

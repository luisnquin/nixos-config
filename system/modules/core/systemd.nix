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

  systemd.extraConfig = ''
    DefaultTimeoutStopSec=15s
  '';
}

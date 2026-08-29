{
  pkgs,
  user,
  ...
}: {
  environment.systemPackages = [pkgs.systemctl-tui];

  security.sudo.extraRules = [
    {
      users = [user.alias];
      commands =
        builtins.map (verb: {
          command = "/run/current-system/sw/bin/systemctl ${verb} *";
          options = ["NOPASSWD"];
        }) [
          "daemon-reload"
          "reload"
          "reload-or-restart"
          "restart"
          "start"
          "stop"
          "try-restart"
        ];
    }
  ];

  systemd.settings.Manager = {
    DefaultTimeoutStopSec = "15s";
  };

  services.journald.extraConfig = ''
    Storage=persistent
    SystemMaxUse=500M
  '';
}

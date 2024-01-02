{
  battery-notifier,
  isTiling,
  host,
  lib,
  ...
}: {
  systemd.user.services = lib.mkIf isTiling {
    battery-notifier = {
      Unit = {
        Description = "A very useful battery notifier for window managers";
      };

      Service = {
        Type = "simple";
        ExecStart = "${battery-notifier}/bin/battery-notifier";
        Restart = "on-failure";
      };

      Install = {
        WantedBy = ["default.target"];
      };
    };
  };
}

{
  config,
  lib,
  pkgs,
  toggle,
  ...
}: {
  "custom/battery" = {
    exec =
      "${lib.getExe pkgs.barfeed.battery}"
      + " --warn ${toString config.services.battery-notifier.settings.warn.threshold}"
      + " --critical ${toString config.services.battery-notifier.settings.threat.threshold}";
    return-type = "json";
    escape = false;
    interval = 3;
    tooltip = true;
    on-click = toggle;
  };
}

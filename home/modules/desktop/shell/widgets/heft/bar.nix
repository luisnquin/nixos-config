{
  config,
  lib,
  pkgs,
  toggle,
  ...
}: {
  "custom/heft" = {
    # No jq here: `heft waybar` is itself a cache read that already emits the
    # bar's JSON, and degrades to a placeholder before the first scan.
    exec =
      "${lib.getExe pkgs.heft} waybar"
      + " --warn-free ${toString config.services.heft.warnFreeGiB}"
      + " --critical-free ${toString config.services.heft.criticalFreeGiB}";
    return-type = "json";
    # the census runs on a timer; polling faster only re-reads a file
    interval = 300;
    tooltip = true;
    on-click = toggle;
  };
}

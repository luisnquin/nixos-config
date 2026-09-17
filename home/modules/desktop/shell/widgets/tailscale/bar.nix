{
  lib,
  pkgs,
  toggle,
  ...
}: {
  "custom/tailscale" = {
    exec = "${lib.getExe pkgs.barfeed.tailscale}";
    return-type = "json";
    restart-interval = 5;
    tooltip = true;
    on-click = toggle;
  };
}

{
  lib,
  pkgs,
  toggle,
  ...
}: {
  "custom/tailscale" = {
    exec = "${lib.getExe pkgs.waytools.tailscale}";
    return-type = "json";
    restart-interval = 5;
    tooltip = true;
    on-click = toggle;
  };
}

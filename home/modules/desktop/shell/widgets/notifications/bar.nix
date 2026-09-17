{
  config,
  lib,
  pkgs,
  name,
  ...
}: let
  # The centre marks itself read on the way out rather than on the way in, so
  # the accents that say "this is new" survive being looked at. That extra step
  # is why this one does not ride the shared toggle.
  notificationsToggle = pkgs.writeShellApplication {
    name = "waybar-notifications";
    runtimeInputs = [config.programs.eww.package pkgs.hark];
    text = ''
      if eww active-windows 2>/dev/null | grep -q "^${name}"; then
        eww close ${name}
        hark seen
        exit 0
      fi
      eww close-all 2>/dev/null || true
      eww open ${name}
    '';
  };
in {
  "custom/notifications" = {
    # No interval: hark streams a fresh line every time mako's D-Bus property
    # is invalidated, so the badge never lags a poll behind.
    exec = "${lib.getExe pkgs.hark} waybar --watch";
    return-type = "json";
    escape = false;
    restart-interval = 5;
    tooltip = true;
    on-click = lib.getExe notificationsToggle;
    on-click-right = "${lib.getExe pkgs.hark} dnd toggle";
    on-click-middle = "${lib.getExe pkgs.hark} clear";
  };
}

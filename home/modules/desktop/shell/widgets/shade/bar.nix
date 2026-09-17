{
  config,
  lib,
  pkgs,
  name,
  ...
}: let
  hark = lib.getExe pkgs.hark;

  # The shade marks itself read on the way out rather than on the way in, so the
  # accents that say "this is new" survive being looked at. That extra step is
  # why the clock does not ride the shared toggle.
  shadeToggle = pkgs.writeShellApplication {
    name = "waybar-shade";
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
  # The clock is the whole notification surface: there is no badge on the bar,
  # so the secondary buttons the old module carried live on here rather than
  # only inside the shade.
  "clock" = {
    interval = 60;
    format = " {:%H:%M}";
    tooltip = true;
    tooltip-format = "{:%A, %B %d %Y}";
    on-click = lib.getExe shadeToggle;
    on-click-right = "${hark} dnd toggle";
    on-click-middle = "${hark} clear";
  };
}

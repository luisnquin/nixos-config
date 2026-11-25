{
  lib,
  pkgs,
  toggle,
  ...
}: {
  "custom/phone" = {
    exec = "${lib.getExe pkgs.phone} grant --waybar";
    return-type = "json";
    escape = false;
    interval = 30;
    signal = pkgs.phone.waybarSignal;
    tooltip = true;
    on-click = toggle;
  };
}

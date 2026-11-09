{
  lib,
  pkgs,
  toggle,
  ...
}: {
  "custom/pile" = {
    exec = "${lib.getExe pkgs.pile} waybar";
    return-type = "json";
    escape = false;
    interval = "once";
    signal = pkgs.pile.signal;
    hide-empty-text = true;
    tooltip = false;
    on-click = toggle;
  };
}

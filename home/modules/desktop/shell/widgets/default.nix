# A widget is one directory: the eww panel, its style, and the waybar module
# that opens it. Not a home-manager module — the eww and waybar modules each
# import this list, so neither one has to name the other's half.
{
  config,
  lib,
  pkgs,
}: let
  ewwPanel = pkgs.writeShellApplication {
    name = "eww-panel";
    runtimeInputs = [config.programs.eww.package];
    text = ''
      target="$1"
      open=$(eww active-windows 2>/dev/null || true)
      eww close-all 2>/dev/null || true
      if ! grep -q "^$target" <<<"$open"; then
        eww open "$target"
      fi
    '';
  };

  load = name: let
    dir = ./. + "/${name}";

    ctx = {
      inherit config lib pkgs name;
      eww = lib.getExe config.programs.eww.package;
      toggle = "${lib.getExe ewwPanel} ${name}";
    };
  in {
    inherit name;
    yuck = (import (dir + "/panel.nix") ctx).yuck;
    style = dir + "/style.scss";
    bar = import (dir + "/bar.nix") ctx;
  };
in
  map load [
    "calendar"
    "sysmon"
    "battery"
    "network"
    "tailscale"
    "heft"
    "notifications"
  ]

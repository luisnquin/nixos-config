{
  config,
  lib,
  pkgs,
  ...
}: let
  ctx = {
    inherit config lib pkgs;
    eww = lib.getExe config.programs.eww.package;
  };

  panels = map (panel: import panel ctx) [
    ./panels/calendar.nix
    ./panels/sysmon.nix
    ./panels/battery.nix
    ./panels/network.nix
    ./panels/tailscale.nix
    ./panels/heft.nix
  ];

  styles = [
    ./style/base.scss
    ./style/calendar.scss
    ./style/sysmon.scss
    ./style/battery.scss
    ./style/network.scss
    ./style/tailscale.scss
    ./style/heft.scss
  ];
in {
  programs.eww = {
    enable = true;
    package = pkgs.eww;

    systemd = {
      enable = true;
      target = "graphical-session.target";
    };

    yuckConfig = lib.concatMapStringsSep "\n" (panel: panel.yuck) panels;
    scssConfig = lib.concatMapStringsSep "\n" builtins.readFile styles;
  };

  systemd.user.services.eww.Service = {
    Restart = "on-failure";
    RestartSec = 5;
  };
}

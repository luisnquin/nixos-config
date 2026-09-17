{
  lib,
  pkgs,
  widgets,
  ...
}: {
  programs.eww = {
    enable = true;
    package = pkgs.eww;

    systemd = {
      enable = true;
      target = "graphical-session.target";
    };

    yuckConfig = lib.concatMapStringsSep "\n" (w: w.yuck) widgets;
    scssConfig = lib.concatMapStringsSep "\n" builtins.readFile ([./eww.scss] ++ map (w: w.style) widgets);
  };

  systemd.user.services.eww.Service = {
    Restart = "on-failure";
    RestartSec = 5;
  };
}

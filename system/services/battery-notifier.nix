{
  battery-notifier,
  host,
  lib,
  ...
}: {
  systemd.services = lib.mkIf (builtins.elem host.desktop ["hyprland" "i3"]) {
    battery-notifier = {
      enable = true;
      description = "A very useful battery notifier for window managers";

      serviceConfig = {
        Type = "simple";
        ExecStart = "${battery-notifier}/bin/battery-notifier";
        Restart = "always";
      };

      # wantedBy = ["graphical-session.target"];
    };
  };
}

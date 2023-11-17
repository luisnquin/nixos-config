{
  battery-notifier,
  host,
  lib,
  ...
}: {
  systemd.user.services = lib.mkIf (builtins.elem host.desktop ["hyprland" "i3"]) {
    battery-notifier = {
      Unit = {
        Description = "A very useful battery notifier for window managers";
      };

      Service = {
        Type = "simple";
        ExecStart = "${battery-notifier}/bin/battery-notifier";
        Restart = "always";
      };

      # wantedBy = ["graphical-session.target"];
    };
  };
}

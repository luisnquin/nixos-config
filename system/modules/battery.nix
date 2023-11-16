{battery-notifier, ...}: {
  systemd.services.battery-notifier = {
    # TODO: only enable this when i3 or hyprland
    enable = true;
    description = "A very useful battery notifier for window managers";

    serviceConfig = {
      Type = "simple";
      ExecStart = "${battery-notifier}/bin/battery-notifier";
      Restart = "always";
    };

    wantedBy = ["graphical-session.target"];
  };
}

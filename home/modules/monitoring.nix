{host, ...}: {
  programs.battery-notifier = {
    enable = host.isLaptop;
    settings = rec {
      interval_ms = 700;
      reminder.threshold = host.batteryThreshold / 2;
      warn.threshold = reminder.threshold / 2;
      threat.threshold = warn.threshold / 3;
    };
  };

  services = {
    playerctld.enable = true;

    udiskie = {
      enable = true;
      automount = true;
      notify = true;
      tray = "auto";
    };
  };
}

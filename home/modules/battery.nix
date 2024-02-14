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
}

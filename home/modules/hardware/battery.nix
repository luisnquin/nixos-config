{host, ...}: {
  services.battery-notifier = {
    enable = host.isLaptop;
    settings = {
      interval_ms = 700;
      reminder.threshold = host.batteryThreshold / 2;
      warn.threshold = 15;
      threat.threshold = 5;
    };
  };
}

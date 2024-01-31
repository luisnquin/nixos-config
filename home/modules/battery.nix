{
  programs.battery-notifier = {
    enable = true;
    settings = {
      interval_ms = 700;
      reminder.threshold = 30;
      threat.threshold = 5;
      warn.threshold = 15;
    };
  };
}

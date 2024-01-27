{
  programs.battery-notifier = {
    enable = true;
    settings = {
      interval_ms = 700;
      reminder_threshold = 30;
      threat_threshold = 5;
      warn_threshold = 15;
    };
  };
}

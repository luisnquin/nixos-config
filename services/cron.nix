{config, ...}
: {
  services.cron = {
    enable = true;
    systemCronJobs = []; # TODO: add notify send service here
  };
}

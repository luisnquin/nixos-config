{
  host,
  user,
  ...
}: {
  services.outage = {
    enable = host.isLaptop;
    user = user.alias;
  };
}

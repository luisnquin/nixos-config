{host, ...}: {
  services.thermald.enable = host.isLaptop;
}

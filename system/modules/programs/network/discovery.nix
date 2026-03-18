{
  services.avahi = {
    enable = true;
    nssmdns4 = true;
    openFirewall = true;

    publish = {
      enable = true;
      addresses = true;
      workstation = true;
      userServices = true;
    };

    denyInterfaces = ["docker0" "veth*"];
  };
}

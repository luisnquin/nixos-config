{
  services.avahi = {
    enable = true;
    ipv6 = false;
    nssmdns4 = true;
    openFirewall = true;

    publish = {
      enable = true;
      addresses = true;
      workstation = true;
      userServices = true;
    };

    allowInterfaces = ["wlo1" "eno2"];

    extraConfig = ''
      [publish]
      publish-a-on-ipv6=no
      publish-aaaa-on-ipv4=no
    '';
  };
}

{host, ...}: {
  services.avahi = {
    enable = true;
    ipv6 = false;
    nssmdns4 = true;
    openFirewall = true;

    publish = {
      enable = true;
      addresses = true;
      workstation = false;
      userServices = true;
    };

    allowInterfaces = ["eno2" "wlo1"];

    extraConfig = ''
      [publish]
      publish-a-on-ipv6=no
      publish-aaaa-on-ipv4=no
    '';

    extraServiceFiles.workstation = ''
      <?xml version="1.0" standalone='no'?>
      <service-group>
        <name replace-wildcards="yes">${host.name}</name>
        <service>
          <type>_workstation._tcp</type>
          <port>9</port>
        </service>
      </service-group>
    '';
  };
}

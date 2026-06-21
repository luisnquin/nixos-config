{
  host,
  pkgs,
  ...
}: {
  # stale pid after switch/restart makes avahi refuse to start (file exists).
  # the hardened unit drops CAP_DAC_OVERRIDE, so a root preStart cannot delete
  # the avahi-owned pid in /run/avahi-daemon (mode 755). the "+" prefix runs the
  # command with full privileges, exempt from the sandbox and bounding set.
  systemd.services.avahi-daemon.serviceConfig.ExecStartPre = "+${pkgs.coreutils}/bin/rm -f /run/avahi-daemon/pid";

  services.avahi = {
    enable = true;
    ipv6 = true;
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

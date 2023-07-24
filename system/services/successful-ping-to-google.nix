{pkgs, ...}: {
  systemd.services.successful-ping-to-google = {
    enable = true;
    description = "Service that works until Google is successfully pinged";
    serviceConfig = {
      Type = "oneshot";
      ExecStart = "${pkgs.bash}/bin/bash -c 'until ${pkgs.iputils}/bin/ping -c1 google.com; do sleep 1; done'";
    };

    after = ["network-online.target"];
    wants = ["network-online.target"];
    wantedBy = ["multi-user.target"];
  };
}

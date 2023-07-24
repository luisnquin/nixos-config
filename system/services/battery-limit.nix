{pkgs, ...}: {
  systemd.services.battery-charge-threshold = {
    enable = true;
    description = "Set the battery charge threshold";
    serviceConfig = {
      Type = "oneshot";
      ExecStart = ''${pkgs.bash}/bin/bash -c "echo 61 > /sys/class/power_supply/BAT1/charge_control_end_threshold"'';
      ExecStop = ''${pkgs.bash}/bin/bash -c "exit 0"'';
    };
    wantedBy = ["multi-user.target"];
  };
}

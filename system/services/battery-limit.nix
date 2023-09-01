{
  pkgs,
  host,
  ...
}: {
  systemd.services.battery-charge-threshold = {
    enable = true;
    description = "Set the battery charge threshold";
    serviceConfig = let
      # threshold = pkgs.lib.strings.toInt host.batteryThreshold;
      # offset = 1;
    in {
      Type = "oneshot";
      # ExecStart = ''${pkgs.bash}/bin/bash -c "echo ${host.batteryThreshold + offset} > /sys/class/power_supply/BAT1/charge_control_end_threshold"'';
      ExecStart = ''${pkgs.bash}/bin/bash -c "echo 61 > /sys/class/power_supply/BAT1/charge_control_end_threshold"'';
    };
    wantedBy = ["multi-user.target"];
  };
}

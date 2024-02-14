{
  pkgs,
  host,
  ...
}: {
  services.thermald.enable = host.isLaptop;

  systemd.services.battery-charge-threshold = {
    enable = host.isLaptop;
    description = "Set the battery charge threshold";
    serviceConfig = let
      offset = 1;

      batteryThreshold = builtins.toString (host.batteryThreshold + offset);
      batteryChargeThresholdPath = "/sys/class/power_supply/BAT1/charge_control_end_threshold";
    in {
      Type = "oneshot";
      ExecStart = ''${pkgs.bash}/bin/bash -c "echo ${batteryThreshold} > ${batteryChargeThresholdPath}"'';
    };
    wantedBy = ["multi-user.target"];
  };
}

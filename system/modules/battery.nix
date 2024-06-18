{
  batlimit,
  pkgs,
  host,
  user,
  ...
}: let
  powerSupplyBatteryPath = "/sys/class/power_supply/BAT1";
in {
  environment.systemPackages = [
    (batlimit.override {inherit powerSupplyBatteryPath;})
  ];

  security.sudo.extraRules = [
    {
      users = [user.alias];
      commands = [
        {
          command = "/run/current-system/sw/bin/batlimit";
          options = ["NOPASSWD"];
        }
      ];
    }
  ];

  services = {
    thermald.enable = host.isLaptop;
    logind.lidSwitchExternalPower = "ignore"; # prevent lid switch from triggering a suspend
  };

  systemd.services.battery-charge-threshold = {
    enable = host.isLaptop;
    description = "Set the battery charge threshold";
    serviceConfig = let
      offset = 1;

      batteryThreshold = builtins.toString (host.batteryThreshold + offset);
      batteryChargeThresholdPath = "${powerSupplyBatteryPath}/charge_control_end_threshold";
    in {
      Type = "oneshot";
      ExecStart = ''${pkgs.bash}/bin/bash -c "echo ${batteryThreshold} > ${batteryChargeThresholdPath}"'';
    };
    wantedBy = ["multi-user.target"];
  };
}

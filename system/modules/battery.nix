{
  host,
  lib,
  ...
}: {
  services = lib.mkIf host.isLaptop {
    thermald.enable = true;

    tlp = {
      enable = true;
      settings = {
        STOP_CHARGE_THRESH_BAT1 = host.batteryThreshold + 1;

        CPU_SCALING_GOVERNOR_ON_AC = "performance";
        CPU_SCALING_GOVERNOR_ON_BAT = "powersave";

        CPU_ENERGY_PERF_POLICY_ON_BAT = "power";
        CPU_ENERGY_PERF_POLICY_ON_AC = "performance";
      };
    };
  };
}

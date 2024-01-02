{
  isTiling,
  host,
  lib,
  ...
}: {
  hardware.bluetooth = lib.mkIf host.bluetooth {
    enable = true;
    powerOnBoot = true;
  };

  services.blueman.enable = host.bluetooth && isTiling;
}

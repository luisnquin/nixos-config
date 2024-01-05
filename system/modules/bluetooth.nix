{
  isTiling,
  config,
  ...
}: {
  hardware.bluetooth = {
    enable = true;
    powerOnBoot = true;
  };

  services.blueman.enable = config.hardware.bluetooth.enable && isTiling;
}

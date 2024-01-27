{
  isTiling,
  config,
  pkgs,
  ...
}: {
  hardware.bluetooth = {
    enable = true;
    powerOnBoot = true;
  };

  environment.systemPackages = [
    pkgs.bluetuith
  ];

  services.blueman.enable = config.hardware.bluetooth.enable && isTiling;
}

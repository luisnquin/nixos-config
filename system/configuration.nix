{
  host,
  nix,
  ...
}: {
  imports = [
    ./services
    ./modules

    ./hardware-configuration.nix
  ];

  tools.nyx = {
    enable = true;
    hyprlandSupport = host.desktop == "hyprland";
  };

  hardware = {
    enableRedistributableFirmware = true;
    cpu.intel.updateMicrocode = true;
  };

  networking = {
    networkmanager.enable = true;
    hostName = host.name;
  };

  time = {
    inherit (host) timeZone;
    # Without this option, the machine will have a UTC time
    hardwareClockInLocalTime = true;
  };

  i18n.defaultLocale = host.i18nLocale;
  location.provider = "geoclue2";

  services.thermald.enable = true;
  services.udisks2.enable = true;

  system = {
    inherit (nix) stateVersion;
    autoUpgrade = {
      enable = true;
      allowReboot = false;
      channel = "https://nixos.org/channels/${nix.channel}";
    };
  };
}

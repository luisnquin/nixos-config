{
  host,
  user,
  nix,
  ...
}: {
  imports = [
    ./hardware-configuration.nix
  ];

  # 🐙
  require = [
    ./../tools

    ./services
    ./modules
  ];

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

  environment.variables = {
    "DOTFILES_PATH" = "/home/${user.alias}/.dotfiles";
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

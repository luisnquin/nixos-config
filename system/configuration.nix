{
  pkgs,
  host,
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
    ./pkgs
  ];

  hardware = {
    enableRedistributableFirmware = true;
    cpu.intel.updateMicrocode = true;

    bluetooth = {
      enable = false;
      powerOnBoot = false;
    };
  };

  # manage realtime priorities
  security.rtkit.enable = true;

  networking = {
    networkmanager.enable = true;
    hostName = host.name;
  };

  time = {
    inherit (host) timeZone;
    # Without this option, the machine will have a UTC time
    hardwareClockInLocalTime = true;
  };

  location.provider = "geoclue2";

  # Chinese
  i18n = {
    defaultLocale = host.i18nLocale;
    inputMethod = {
      enabled = "ibus";
      ibus = {
        engines = with pkgs.ibus-engines; [
          libpinyin
        ];
      };
    };
  };
  # zh-CN

  sound.enable = true;

  services = {
    thermald.enable = true;

    # pulseaudio doesn't give a good support for some programs
    pipewire = {
      enable = true;
      alsa.enable = true;
      alsa.support32Bit = true;
      pulse.enable = true;
      jack.enable = true;
    };
  };

  system = {
    inherit (nix) stateVersion;
    autoUpgrade = {
      enable = true;
      allowReboot = false;
      channel = "https://nixos.org/channels/${nix.channel}";
    };
  };
}

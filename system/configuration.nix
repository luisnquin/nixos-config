{
  pkgs,
  host,
  ...
}: {
  imports = [
    ./hardware-configuration.nix
  ];

  # 🐙
  require = [
    ./services
    ./modules
    ./pkgs
  ];

  boot = with pkgs; {
    loader = {
      efi.canTouchEfiVariables = true;
      timeout = 15;

      grub = let
        falloutTheme =
          pkgs.fetchFromGitHub
          {
            owner = "shvchk";
            repo = "fallout-grub-theme";
            rev = "80734103d0b48d724f0928e8082b6755bd3b2078";
            sha256 = "sha256-7kvLfD6Nz4cEMrmCA9yq4enyqVyqiTkVZV5y4RyUatU=";
          };
      in {
        gfxmodeBios = host.resolution;
        gfxmodeEfi = host.resolution;
        theme = falloutTheme;

        enable = true;
        device = "nodev";
        useOSProber = true;
        efiSupport = true;
        forceInstall = false;
      };
    };

    kernelPackages = linuxPackages_latest;
    supportedFilesystems = ["ntfs"];
    tmp.cleanOnBoot = true;
  };

  hardware = {
    enableRedistributableFirmware = true;
    cpu.intel.updateMicrocode = true;

    opengl = {
      enable = true;
      driSupport32Bit = true;
    };

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
    stateVersion = "23.05";
    autoUpgrade = {
      enable = true;
      allowReboot = false;
      channel = "https://nixos.org/channels/nixos-unstable";
    };
  };
}

{
  config,
  pkgs,
  ...
}: let
  owner = import "/etc/nixos/owner.nix";
in {
  imports = [
    <home-manager/nixos>

    ./hardware-configuration.nix
  ];

  # Entry point 🐙
  require = [
    "/etc/nixos/services/default.nix"
    "/etc/nixos/modules/default.nix"
    "/etc/nixos/pkgs/default.nix"
    "/etc/nixos/home.nix"
  ];

  boot = with pkgs; {
    loader = {
      efi.canTouchEfiVariables = true;
      timeout = 8;

      grub = let
        resolution = "1920x1080";
      in {
        theme = "/etc/nixos/dots/boot/grub/themes/catppuccin-mocha-grub-theme";
        gfxmodeBios = resolution;
        gfxmodeEfi = resolution;

        enable = true;
        version = 2;
        device = "nodev";
        useOSProber = true;
        efiSupport = true;
        forceInstall = false;
      };
    };

    kernelPackages = linuxPackages_latest;
    supportedFilesystems = ["ntfs"];
    cleanTmpDir = true;
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
    hostName = "nyx";
  };

  time = {
    timeZone = "America/Lima";
    # Without this option, the machine will have a UTC time
    hardwareClockInLocalTime = true;
  };

  location.provider = "geoclue2";

  fonts = {
    fonts = with pkgs; [
      (nerdfonts.override {fonts = ["FiraCode" "CascadiaCode" "NerdFontsSymbolsOnly"];})
      cascadia-code
      jetbrains-mono
      roboto-mono
      inconsolata
      roboto
    ];

    fontDir.enable = true;
  };

  # Chinese
  i18n = {
    defaultLocale = "en_US.UTF-8";
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
      enable = false;
      allowReboot = false;
      channel = "https://nixos.org/channels/nixos-23.05";
    };
  };
}

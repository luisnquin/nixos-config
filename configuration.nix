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

      grub = let
        resolution = "1920x1080";
      in {
        enable = true;
        version = 2;
        device = "nodev";
        useOSProber = true;
        efiSupport = true;
        forceInstall = false;

        # Theming
        theme = "/etc/nixos/dots/boot/grub/themes/catppuccin-mocha-grub-theme";
        # the default value of these two sucks
        gfxmodeBios = resolution;
        gfxmodeEfi = resolution;
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

    pulseaudio = {
      enable = false;
      package = pkgs.pulseaudioFull;

      # ref https://github.com/NixOS/nixpkgs/issues/71362#issuecomment-753461502
      extraConfig = ''
        unload-module module-native-protocol-unix
        load-module module-native-protocol-unix auth-anonymous=1
      '';
    };
  };

  security = {
    # hands out realtime scheduling priority to user processes on demand
    rtkit.enable = true;

    sudo = {
      enable = true;
      wheelNeedsPassword = true;
    };
  };

  users = {
    motd = "It's a good moment to tell you that this will be a great day for you 🌇";

    users = with owner; {
      ${username} = {
        isNormalUser = true;
        home = ''/home/${username}/'';
        # Used by desktop manager
        description = ''${username} 🌂'';
        shell = pkgs.zsh;
        hashedPassword = null;
        # ❄️

        extraGroups = [
          "wheel"
          "docker"
        ];
      };
    };
  };

  networking = {
    networkmanager.enable = true;
    hostName = "nyx";

    firewall = {
      enable = true;
      allowedTCPPorts = [20 80 443 8088];
      allowPing = false;
    };
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

  i18n.defaultLocale = "es_PE.UTF-8";

  sound.enable = true;

  services = {
    gnome.gnome-keyring.enable = true;
    thermald.enable = true;

    openssh = {
      enable = true;
      banner = "Hiiii how'r u, plz let me in";
      settings.passwordAuthentication = true;

      knownHosts = let
        primaryPublicKey = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIJcSOpun+OjJng87LUXArDX3y2LLts7pOpfyCC1Mygew luisnquin@rat";
      in {
        "https://github.com" = {
          publicKey = primaryPublicKey;
        };
        "https://gitlab.com" = {
          publicKey = primaryPublicKey;
        };
      };
    };

    # pulseaudio doesn't give a good support for some programs
    pipewire = {
      enable = true;
      alsa.enable = true;
      alsa.support32Bit = true;
      pulse.enable = true;
      jack.enable = true;
    };
  };

  # sway.enable = true;
  programs.gnupg.agent = {
    enable = true;
    enableSSHSupport = true;
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

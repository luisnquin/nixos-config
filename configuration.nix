{
  config,
  pkgs,
  ...
}: let
  username = "luisnquin";
  spotify_username = "yeselony";
in {
  imports = [
    ./hardware-configuration.nix
  ];

  require = [
    "/etc/nixos/modules/default.nix"
    "/etc/nixos/units/default.nix"
    "/etc/nixos/pkgs/default.nix"
  ];

  nixpkgs.config = {
    allowBroken = false;
    allowUnfree = true;
  };

  nix = {
    gc = {
      automatic = true;
      dates = "weekly";
      options = "--delete-old";
    };

    # Nix store
    optimise = {
      automatic = true;
      dates = ["13:00"];
    };

    settings = {
      auto-optimise-store = true;
      experimental-features = ["nix-command" "flakes"];
      max-jobs = 4;
    };
  };

  boot = {
    loader = {
      efi.canTouchEfiVariables = true;

      grub = {
        enable = true;
        version = 2;
        device = "nodev";
        useOSProber = true;
        efiSupport = true;
      };
    };

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

  security.sudo = {
    enable = true;
    wheelNeedsPassword = true;
  };

  users = {
    motd = "It's a good moment to tell you that this will be a great day for you 🌇";

    users = {
      ${username} = {
        isNormalUser = true;
        home = ''/home/${username}/'';
        description = ''${username} computer'';
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
      trustedInterfaces = ["docker0"];
    };
  };

  console = {
    font = "Lat2-Terminus16";
    keyMap = "es";
  };

  time = {
    timeZone = "America/Lima";
    # Without this option, the machine will have a UTC time
    hardwareClockInLocalTime = true;
  };

  fonts = {
    fonts = with pkgs; [
      cascadia-code
      jetbrains-mono
      (nerdfonts.override {fonts = ["FiraCode" "CascadiaCode"];})
    ];

    fontDir.enable = true;
  };

  virtualisation.docker = {
    enableNvidia = true;
    enable = true;

    autoPrune = {
      enable = true;
      dates = "daily";
    };
  };

  i18n.defaultLocale = "es_PE.UTF-8";
  xdg.portal.wlr.enable = true;
  sound.enable = true;

  systemd.services.spotifyd = {
    after = ["network-online.target" "sound.target"];
    serviceConfig = {
      SupplementaryGroups = ["audio"];
      DynamicUser = true;
      User = ''${username}'';
      Restart = "always";
    };
  };

  services = {
    gnome.gnome-keyring.enable = true;
    thermald.enable = true;

    openssh = {
      enable = true;
      passwordAuthentication = true;
    };

    spotifyd = {
      enable = true;
      settings.global = {
        # use_keyring = true;
        autoplay = true;
        # backend = "pulseaudio";
        bitrate = 320;
        cache_path = ''/home/${username}/.cache/spotifyd'';
        # dbus_type = "session";
        device_name = config.networking.hostName;
        device_type = "computer";
        initial_volume = "80";
        max_cache_size = 1000000000;
        no_audio_cache = false;
        # TODO: implementation for secrets and other personal settings
        password = "";
        # use_mpris = true;
        username = ''${spotify_username}''; # Thanks to the guy who stole and changed my username 5 years ago
        volume_normalisation = false;
      };
    };

    xserver = {
      videoDrivers = ["nvidia"];
      libinput.enable = true;
      layout = "latam";
      autorun = true;
      enable = true;

      displayManager = {
        gdm.enable = true;
        startx.enable = true;
        defaultSession = "plasma";
      };

      desktopManager = {
        plasma5.enable = true;
        xterm.enable = true;
      };
    };

    # pulseaudio doesn't give a good support for some programs
    pipewire = {
      enable = true;
      alsa.enable = true;
      pulse.enable = true;
      jack.enable = true;
    };
  };

  programs = {
    # sway.enable = true;
    # mtr.enable = true;

    nano.nanorc = ''
      set titlecolor white,magenta
      set positionlog
      set autoindent
      set tabsize 4
      set atblanks
      set zero
    '';

    gnupg.agent = {
      enable = true;
      enableSSHSupport = true;
    };

    zsh = {
      enable = true;
      enableCompletion = true;
      autosuggestions.enable = true;
      syntaxHighlighting.enable = true;
    };
  };

  system = {
    stateVersion = "22.11";
    autoUpgrade = {
      enable = true;
      allowReboot = false;
      channel = "https://nixos.org/channels/nixos-22.11";
    };
  };
}

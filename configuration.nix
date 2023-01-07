{
  config,
  pkgs,
  ...
}: let
  owner = import "/etc/nixos/owner.nix";
in {
  imports = [
    ./hardware-configuration.nix
    # https://nix-community.github.io/home-manager/index.html#ch-installation
    <home-manager/nixos>
  ];

  require = [
    "/etc/nixos/modules/default.nix"
    "/etc/nixos/units/default.nix"
    "/etc/nixos/pkgs/default.nix"
    "/etc/nixos/home.nix"
  ];

  nixpkgs.config = {
    permittedInsecurePackages = [
      "electron-12.2.3"
    ];
    allowBroken = false;
    # The day I meet the man who has this option in false
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
        font = ''${pkgs.grub2}/share/grub/unicode.pf2'';
        fontSize = 12;
        theme = pkgs.fetchFromGitHub {
          owner = "shvchk";
          repo = "fallout-grub-theme";
          rev = "80734103d0b48d724f0928e8082b6755bd3b2078";
          sha256 = "sha256-7kvLfD6Nz4cEMrmCA9yq4enyqVyqiTkVZV5y4RyUatU=";
        };
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

  security = {
    rtkit.enable = true;

    sudo = {
      enable = true;
      wheelNeedsPassword = true;
    };
  };

  users = {
    motd = "It's a good moment to tell you that this will be a great day for you 🌇";

    users = {
      ${owner.username} = {
        isNormalUser = true;
        home = ''/home/${owner.username}/'';
        # Used by desktop manager
        description = ''${owner.username} 🌂'';
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
      inconsolata # Used by zathura
      (nerdfonts.override {fonts = ["FiraCode" "CascadiaCode"];})
    ];

    fontDir.enable = true;
  };

  xdg = {
    portal.wlr.enable = true;

    # TODO: fix and add more kv's here
    mime.defaultApplications = {
      "text/csv" = "code.desktop"; # Suspect
      "text/x-go" = "code.desktop"; # of
      "video/x-matroska" = "vlc.desktop;vlc-2.desktop"; # these
      "x-scheme-handler/postman" = "Postman.desktop";
      "x-scheme-handler/slack" = "slack.desktop";
      "x-scheme-handler/http" = "brave.desktop";
      "application/pdf" = ["zathura.desktop" "brave.desktop"];
      "image/png" = ["gwenview.desktop" "gimp.desktop"];
    };
  };

  i18n.defaultLocale = "es_PE.UTF-8";
  sound.enable = true;

  services = {
    gnome.gnome-keyring.enable = true;
    thermald.enable = true;

    openssh = {
      enable = true;
      passwordAuthentication = true;
    };

    xserver = {
      videoDrivers = ["nvidia"];
      libinput.enable = true;
      layout = "latam";
      autorun = true;
      enable = true;

      displayManager = {
        gdm = {
          enable = true;
          autoSuspend = false;
        };

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
      alsa.support32Bit = true;
      pulse.enable = true;
      jack.enable = true;
    };
  };

  programs = {
    # sway.enable = true;
    # mtr.enable = true;

    nano = {
      nanorc = ''
        set titlecolor white,magenta
        set positionlog
        set autoindent
        set tabsize 4
        set atblanks
        set zero
      '';

      syntaxHighlight = true;
    };

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
      enable = false;
      allowReboot = false;
      channel = "https://nixos.org/channels/nixos-22.11";
    };
  };
}

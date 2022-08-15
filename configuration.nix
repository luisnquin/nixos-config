{
  config,
  pkgs,
  ...
}: {
  imports = [
    ./hardware-configuration.nix
  ];

  nixpkgs.config = {
    allowBroken = false;
    allowUnfree = true;
  };

  nix = {
    # Garbage collector
    gc = {
      automatic = true;
      dates = "13:00";
    };

    # Nix store
    optimise = {
      automatic = true;
      dates = ["13:00"];
    };

    settings = {
      # Nix automatically detects files in the store that have identical contents, and replaces them with hard links to a single copy.
      auto-optimise-store = true;
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

    /*
     initrd.preLVMCommands = ''
       echo '--- OWNERSHIP NOTICE ---'
       echo 'This device is property of Luis Quiñones Requelme'
       echo 'If lost please contact to lpaandres2020@gmail.com'
       echo '--- OWNERSHIP NOTICE ---'
     '';
     */

    cleanTmpDir = true;
    supportedFilesystems = ["ntfs"];
  };

  hardware = {
    enableRedistributableFirmware = true;
    opengl.enable = true;

    bluetooth = {
      enable = false;
      powerOnBoot = false;
    };

    pulseaudio = {
      enable = true;
      # package = pkgs.pulseaudioFull; Uncomment it in case of start using bluetooth
    };

    nvidia = {
      package = config.boot.kernelPackages.nvidiaPackages.beta;
      powerManagement.enable = true;
      modesetting.enable = true;
      prime = {
        offload.enable = true;
        intelBusId = "PCI:0:2:0";
        nvidiaBusId = "PCI:1:0:0";
      };
    };
  };

  security.sudo = {
    enable = true;
    wheelNeedsPassword = true;
  };

  users = {
    users.luisnquin = {
      isNormalUser = true;
      home = "/home/luisnquin";
      description = "Luis Quiñones";
      shell = pkgs.zsh;
      # ❄️

      extraGroups = [
        "wheel"
        "docker"
      ];
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

  time.timeZone = "America/Lima";

  fonts = {
    fonts = with pkgs; [
      cascadia-code
      jetbrains-mono
      (nerdfonts.override {fonts = ["FiraCode" "CascadiaCode"];})
    ];

    fontDir.enable = true;
  };

  virtualisation = {
    docker = {
      enable = true;
      autoPrune = {
        enable = true;
        dates = "weekly";
      };
    };
  };

  i18n.defaultLocale = "es_PE.UTF-8";
  xdg.portal.wlr.enable = true;
  sound.enable = true;

  services = {
    gnome.gnome-keyring.enable = true;
    thermald.enable = true;
    openssh.enable = true;

    xserver = {
      videoDrivers = ["nvidia"];
      libinput.enable = true;
      layout = "latam";
      autorun = true;
      enable = true;

      displayManager = {
        gdm.enable = true;
        startx.enable = true;
      };

      desktopManager = {
        plasma5.enable = true;
        xterm.enable = true;
      };
    };
  };

  programs = {
    sway.enable = true;
    mtr.enable = true;

    tmux = {
      enable = true;
      clock24 = true;
      terminal = "xterm-256color";
      extraConfig = ''
        setw -g mouse on
        set -g window-status-separator ""

        set-option -ga terminal-overrides ",*256col*:Tc:RGB"

        set -g status-bg black
        set -g status-fg magenta

        set -g status-left-length 40
        set -g status-left "#S #[fg=white]#[fg=yellow]#I #[fg=cyan]#P"
      '';

      historyLimit = 1000000;
      newSession = false;
    };

    /*
     gnu.nano = {
       extraConfig = ''
         set titlecolor white,magenta
         set autoindent
         set tabsize 4
         set atblanks
         set zero
       '';
     };
     */

    gnupg.agent = {
      enable = true;
      enableSSHSupport = true;
    };

    # Consider starting to use 'fish' on a NixOS instance from scratch
    # fish.enable = true;

    zsh = {
      enable = true;
      enableCompletion = true;
      autosuggestions.enable = true;
      syntaxHighlighting.enable = true;

      shellAliases = {
        runds = "rm -rf compose/nginx/env.json && make compose-up && make build && make run";
        v3 = "cd ~/go/src/gitlab.wiserskills.net/wiserskills/v3/";
        kritapl = "cd ~/workspace/projects/illustrations/";
        dot = "cd ~/.dotfiles/";

        playground = "cd ~/workspace/playground/";
        projects = "cd ~/workspace/projects/";
        tests = "cd ~/workspace/tests/";
        workspace = "cd ~/workspace/";
        etc = "cd ~/.etc/";

        nyx = "sh ~/.dotfiles/.scripts/main.sh";
        xclip = "xclip -selection c";
        ale = "alejandra";
        open = "xdg-open";
        py = "python3";
        cat = "bat -p";
      };
    };
  };

  environment = {
    systemPackages = with pkgs; let
      set = {
        nix = with pkgs; [vscode-extensions.jnoortheen.nix-ide alejandra rnix-lsp];
        apps = with pkgs; [spotify discord vscode slack fragments brave];
        # kubernetes = with pkgs; [kubectl kubernetes minikube];
        go = with pkgs; [go_1_18 gopls gofumpt delve gcc];
        # rust = with pkgs; [cargo rustc rustup rustfmt];
        docker = with pkgs; [docker docker-compose];
        python = with pkgs; [python310 virtualenv];
        node = with pkgs; [nodejs-18_x];
        yard = with pkgs; [krita];
        dev = with pkgs; [
          nodePackages.firebase-tools
          pre-commit
          redoc-cli
          shfmt
          sqlc
          tmux
          git
        ];
      };

      nvidia-offload = pkgs.writeShellScriptBin "nvidia-offload" ''
        export __NV_PRIME_RENDER_OFFLOAD=1
        export __NV_PRIME_RENDER_OFFLOAD_PROVIDER=NVIDIA-G0
        export __GLX_VENDOR_LIBRARY_NAME=nvidia
        export __VK_LAYER_NV_optimus=NVIDIA_only
        exec "$@"
      '';
    in
      [
        gnome.seahorse
        nvidia-offload
        binutils
        openjdk
        gnumake
        openssh
        ntfs3g
        neovim
        unzip
        exfat
        xclip
        wget
        dpkg
        tree
        unar
        vim
        bat
        zip
        zsh
        jq
      ]
      # ++ set.kubernetes
      ++ set.python
      ++ set.docker
      ++ set.yard
      ++ set.node
      ++ set.apps
      # ++ set.rust
      ++ set.dev
      ++ set.nix
      ++ set.go;

    sessionVariables = rec {
      PATH = "$GORROT:$GOPATH/bin:$PATH";
      CGO_ENABLED = "0";
    };

    variables = {
      EDITOR = "nano";
    };

    interactiveShellInit = ''
      if [ "$TMUX" = "" ] && [ "$TERM_PROGRAM" != "vscode" ] ; then exec tmux; fi
    '';
  };

  systemd.services = {
    batteryChargeThreshold = {
      enable = true;
      description = "Set the battery charge threshold";
      serviceConfig = {
        Type = "oneshot";
        ExecStart = ''${pkgs.bash}/bin/bash -c "echo 61 > /sys/class/power_supply/BAT1/charge_control_end_threshold"'';
        ExecStop = ''${pkgs.bash}/bin/bash -c "exit 0"'';
      };
      wantedBy = ["multi-user.target"];
    };
  };

  system = {
    stateVersion = "22.11";
    autoUpgrade = {
      enable = true;
      allowReboot = true;
      channel = "https://nixos.org/channels/nixos-22.11";
    };
  };
}

{
  config,
  pkgs,
  ...
}: {
  imports = [./hardware-configuration.nix];

  nixpkgs.config = {
    android_sdk.accept_license = true;
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

    # Nix automatically detects files in the store that have identical contents, and replaces them with hard links to a single copy.
    settings = {
      auto-optimise-store = true;
      max-jobs = "auto";
    };
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

    cleanTmpDir = true;
    supportedFilesystems = ["ntfs"];
  };

  hardware = {
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
    wheelNeedsPassword = false;
  };

  users = {
    users.luisnquin = {
      isNormalUser = true;
      home = "/home/luisnquin";
      description = "Luis Quiñones";
      shell = pkgs.zsh;
      # ❄️

      extraGroups = ["wheel" "docker" "adbusers"];
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

  console = {
    font = "Lat2-Terminus16";
    keyMap = "es";
  };

  services = {
    gnome.gnome-keyring.enable = true;
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

  environment.systemPackages = with pkgs; let
    set = {
      nix = with pkgs; [vscode-extensions.jnoortheen.nix-ide alejandra rnix-lsp];
      apps = with pkgs; [spotify discord vscode slack fragments];
      kubernetes = with pkgs; [kubectl kubernetes minikube];
      go = with pkgs; [go_1_18 gopls gofumpt delve gcc];
      android = with pkgs; [android-tools flutter dart];
      rust = with pkgs; [cargo rustc rustup rustfmt];
      docker = with pkgs; [docker docker-compose];
      python = with pkgs; [python310 virtualenv];
      node = with pkgs; [nodejs-18_x];
      db = with pkgs; [postgresql];
      dev = with pkgs; [
        nodePackages.firebase-tools
        pre-commit
        redoc-cli
        shfmt
        sqlc
        tmux
        sass
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
      stow
      dpkg
      tree
      unar
      vim
      bat
      zip
      zsh
      jq
    ]
    ++ set.android
    ++ set.python
    ++ set.docker
    ++ set.node
    ++ set.apps
    ++ set.rust
    ++ set.dev
    ++ set.nix
    ++ set.go
    ++ set.db;

  programs = {
    sway.enable = true;
    adb.enable = true;
    mtr.enable = true;

    tmux = {
      enable = true;
      clock24 = true;
      terminal = "xterm-256color";
      extraConfig = ''
        setw -g mouse on
        set -g history-limit 1000000

        set-option -ga terminal-overrides ",*256col*:Tc:RGB"

        set -g status-bg black
        set -g status-fg magenta

        set -g status-left-length 40
        set -g status-left "#S #[fg=white]#[fg=yellow]#I #[fg=cyan]#P"
      '';

      newSession = false;
      historyLimit = 1000000;
    };

    /*
     gnupg.nano = {
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

    bash = {
      enableCompletion = true;
    };

    zsh = {
      enable = true;
      enableCompletion = true;
      autosuggestions.enable = true;
      syntaxHighlighting.enable = true;

      shellAliases = {
        sysclean = "sudo nix-collect-garbage -d; and sudo nix-store --optimise";
        runds = "rm -rf compose/nginx/env.json && make compose-up && make build && make run";
        v3 = "cd ~/go/src/gitlab.wiserskills.net/wiserskills/v3";
        dot = "cd ~/.dotfiles";

        playground = "cd ~/workspace/playground";
        projects = "cd ~/workspace/projects";
        tests = "cd ~/workspace/tests";
        workspace = "cd ~/workspace";
        etc = "cd ~/.etc/";

        xclip = "xclip -selection c";
        ale = "alejandra";
        open = "xdg-open";
        py = "python3";
        cat = "bat -p";
      };
    };
  };

  environment = {
    sessionVariables = rec {
      GOPRIVATE = "gitlab.wiserskills.net/wiserskills/";
      PATH = "$GORROT:$GOPATH/bin:$PATH";
      GO111MODULE = "on";
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
    stateVersion = "22.05";
    autoUpgrade = {
      enable = true;
      allowReboot = true;
      channel = "https://nixos.org/channels/nixos-22.05";
    };
  };
}

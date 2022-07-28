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

  time.timeZone = "America/Lima";

  fonts.fonts = with pkgs; [cascadia-code jetbrains-mono nerdfonts];

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
  };

  system.autoUpgrade = {
    enable = true;
    allowReboot = true;
    channel = "https://nixos.org/channels/nixos-22.05";
  };

  hardware = {
    pulseaudio.enable = true;
    opengl.enable = true;

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
    docker.enable = true;
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
      nix = with pkgs; [vscode-extensions.jnoortheen.nix-ide alejandra];
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
      newSession = true;
      historyLimit = 1000000;
    };

    gnupg.agent = {
      enable = true;
      enableSSHSupport = true;
    };

    bash = {
      enableCompletion = true;
    };

    zsh = {
      enable = true;
      autosuggestions.enable = true;
      enableCompletion = true;
      syntaxHighlighting.enable = true;
    };
  };

  environment = {
    sessionVariables = rec {
      GOPRIVATE = "gitlab.wiserskills.net/wiserskills/";
      PATH = "$GORROT:$GOPATH/bin:$PATH";
      GO111MODULE = "on";
    };

    interactiveShellInit = ''
      alias v3='cd ~/go/src/gitlab.wiserskills.net/wiserskills/v3'
      alias dot='cd ~/.dotfiles'

      alias playground='cd ~/workspace/playground'
      alias projects='cd ~/workspace/projects'
      alias tests='cd ~/workspace/tests'
      alias workspace='cd ~/workspace'
      alias etc='cd ~/.etc/'

      alias ale='alejandra'
      alias open='xdg-open'
      alias py='python3'
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

  system.stateVersion = "22.05";
}

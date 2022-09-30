{
  config,
  pkgs,
  ...
}: {
  imports = [
    ./hardware-configuration.nix
  ];

  require = [
    "/etc/nixos/units/battery-limit.nix"
    "/etc/nixos/modules/nvidia.nix"
    "/etc/nixos/modules/tmux.nix"
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
    opengl.enable = true;

    bluetooth = {
      enable = false;
      powerOnBoot = false;
    };

    pulseaudio = {
      enable = false;
      package = pkgs.pulseaudioFull;
    };
  };

  security.sudo = {
    enable = true;
    wheelNeedsPassword = true;
  };

  users = {
    users = {
      luisnquin = {
        isNormalUser = true;
        home = "/home/luisnquin/";
        description = "Luis Quiñones";
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

  time.timeZone = "America/Lima";

  fonts = {
    fonts = with pkgs; [
      cascadia-code
      jetbrains-mono
      (nerdfonts.override {fonts = ["FiraCode" "CascadiaCode"];})
    ];

    fontDir.enable = true;
  };

  virtualisation.docker = {
    enable = true;
    autoPrune = {
      enable = true;
      dates = "daily";
    };
  };

  i18n.defaultLocale = "es_PE.UTF-8";
  xdg.portal.wlr.enable = true;
  sound.enable = true;

  services = {
    gnome.gnome-keyring.enable = true;
    thermald.enable = true;

    openssh = {
      enable = true;
      passwordAuthentication = true;
    };

    xserver = {
      videoDrivers = ["nvidia"]; # Ternary?
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

      # windowManager = {};
    };

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

  environment = {
    systemPackages = with pkgs; let
      set = {
        # kubernetes = with pkgs; [kubectl kubernetes minikube];

        rust = with pkgs; [
          rustfmt
          rustup
          cargo
          rustc
        ];

        nix = with pkgs; [
          vscode-extensions.jnoortheen.nix-ide
          alejandra
          rnix-lsp
        ];

        apps = with pkgs; [
          fragments
          spotify
          discord
          vscode
          slack
          brave
        ];

        go = with pkgs; [
          go_1_18
          gofumpt
          gotools
          gopls
          delve
          gcc
        ];

        docker = with pkgs; [
          docker-compose
          docker
        ];

        python = with pkgs; [
          virtualenv
          python310
        ];

        js = with pkgs; [
          nodejs-18_x
          nodePackages.vue-cli
        ];

        yard = with pkgs; [
          krita
        ];

        dev = with pkgs; [
          nodePackages.firebase-tools
          pre-commit
          redoc-cli
          websocat
          dbeaver
          shfmt
          sqlc
          tmux
          git
        ];
      };
    in
      [
        gnome.seahorse
        binutils
        openjdk
        gnumake
        openssh
        ntfs3g
        neovim
        gotop
        unzip
        exfat
        xclip
        wget
        dpkg
        tree
        unar
        vim
        vlc
        bat
        zip
        zsh
        jq
      ]
      # ++ set.kubernetes
      # ++ set.rust
      ++ set.python
      ++ set.docker
      ++ set.yard
      ++ set.rust
      ++ set.apps
      ++ set.dev
      ++ set.nix
      ++ set.js
      ++ set.go;

    shellAliases = {
      # Complex script aliases

      ## Git shortcuts
      gl1 = "git log --graph --abbrev-commit --decorate --format=format:'%C(bold blue)%h%C(reset) - %C(bold green)(%ar)%C(reset) %C(white)%s%C(reset) %C(dim white)- %an%C(reset)%C(bold yellow)%d%C(reset)' --all";
      gl2 = "git log --graph --abbrev-commit --decorate --format=format:'%C(bold blue)%h%C(reset) - %C(bold cyan)%aD%C(reset) %C(bold green)(%ar)%C(reset)%C(bold yellow)%d%C(reset)%n'' %C(white)%s%C(reset) %C(dim white)- %an%C(reset)' --all";
      gsp = "git stash pop";
      gs = "git stash";
      gl = "gl1";

      ## Computer manager
      nyx = "sh ~/.dotfiles/.scripts/main.sh";

      ## ???
      runds = "rm -rf compose/nginx/env.json && make compose-up && make build && make run";
      v3 = "cd ~/go/src/gitlab.wiserskills.net/wiserskills/v3/";
      ds = "v3 && cd dataserver/";

      playground = "cd ~/workspace/playground/";
      pypl = "cd ~/workspace/playground/python/";
      gopl = "cd ~/workspace/playground/go/";
      pl = "playground";

      projects = "cd ~/workspace/projects/";
      pr = "projects";

      tests = "cd ~/workspace/tests/";
      workspace = "cd ~/workspace/";
      down = "cd ~/Downloads/";
      dot = "cd ~/.dotfiles/";
      etc = "cd ~/.etc/";

      # etc
      xclip = "xclip -selection c";
      gotop = "gotop --nvidia";
      wscat = "websocat";
      ale = "alejandra";
      open = "xdg-open";
      py = "python3";
      cat = "bat -p";
    };

    sessionVariables = rec {
      GOPRIVATE = "gitlab.wiserskills.net/wiserskills/";
      PATH = "$PATH:$GORROT:$GOPATH/bin";
      GOPATH = "/home/$USER/go";
      CGO_ENABLED = "0";
    };

    variables = {
      EDITOR = "nano";
    };

    interactiveShellInit = ''
      if [ "$TMUX" = "" ] && [ "$TERM_PROGRAM" != "vscode" ] ; then exec tmux; fi
    '';
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

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
    "/etc/nixos/services/cron.nix"
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

    # enableNvidia = true;
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
      nyxPkgs = {
        kubernetes = with pkgs; [
          kubernetes
          minikube
          kubectl
          k9s
        ];

        apps = with pkgs; [
          brave
          discord
          fragments
          slack
          spotify
          vscode
        ];

        go = with pkgs; [
          air
          delve
          gcc
          go_1_19
          gofumpt
          gopls
          gotools
        ];

        rust = with pkgs; [
          cargo
          clippy
          rustc
          rust-analyzer
          rustfmt
          rustup
        ];

        js = with pkgs; [
          nodePackages.pnpm
          nodejs-18_x
        ];

        nix = with pkgs; [
          alejandra
          rnix-lsp
          vscode-extensions.jnoortheen.nix-ide
        ];

        git = with pkgs; [
          git
          lazygit
          pre-commit
        ];

        docker = with pkgs; [
          docker
          docker-compose
          lazydocker
        ];

        python = with pkgs; [
          python310
          virtualenv
        ];

        dev = with pkgs; [
          nodePackages.firebase-tools
          nodePackages.prettier
          onlyoffice-bin
          obs-studio
          redoc-cli
          gomplate
          websocat
          dbeaver
          shfmt
          ngrok
          sqlc
          lens
          tmux
        ];
      };
    in
      [
        gnome.gnome-sound-recorder
        gnome.seahorse
        imagemagick
        libnotify
        binutils
        neofetch
        tenacity
        openjdk
        gnumake
        fortune
        thefuck
        openssh
        cowsay
        ffmpeg
        ntfs3g
        neovim
        lolcat
        gotop
        p7zip
        unzip
        krita
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
      ++ nyxPkgs.kubernetes
      ++ nyxPkgs.python
      ++ nyxPkgs.docker
      ++ nyxPkgs.rust
      ++ nyxPkgs.apps
      ++ nyxPkgs.dev
      ++ nyxPkgs.git
      ++ nyxPkgs.nix
      ++ nyxPkgs.js
      ++ nyxPkgs.go;

    shellAliases = {
      ## Git shortcuts
      g = "git";
      ga = "git add";
      gaa = "git add --all";
      gb = "git branch";
      gc = "git commit -v";
      gca = "git commit --amend";
      gck = "git checkout";
      gd = "git diff";
      gds = "git diff --staged";
      gl = "git log --oneline";
      gl1 = "git log --graph --abbrev-commit --decorate --format=format:'%C(bold blue)%h%C(reset) - %C(bold green)(%ar)%C(reset) %C(white)%s%C(reset) %C(dim white)- %an%C(reset)%C(bold yellow)%d%C(reset)' --all";
      gl2 = "git log --graph --abbrev-commit --decorate --format=format:'%C(bold blue)%h%C(reset) - %C(bold cyan)%aD%C(reset) %C(bold green)(%ar)%C(reset)%C(bold yellow)%d%C(reset)%n'' %C(white)%s%C(reset) %C(dim white)- %an%C(reset)' --all";
      gpull = "git pull origin";
      gpullf = "gpull -f";
      gpush = "git push origin";
      gpushf = "gpush -f";
      gr = "git reset";
      grb = "git rebase";
      grba = "git rebase --abort";
      grbc = "git rebase --continue";
      gs = "git stash";
      gsp = "git stash pop";
      gss = "git status -s";
      gt = "git tag";

      # Docker shortcuts
      dka = "docker kill $(docker ps -qa) 2> /dev/null";
      dra = "docker rm $(docker ps -qa) 2> /dev/null";
      dria = "docker rmi -f $(docker image -qa)";
      di = "docker image ls";
      dss = "docker stats";
      dpa = "docker ps -a";
      dp = "docker ps";
      dc = "docker cp";

      # ???
      nyx = "sh ~/.dotfiles/.scripts/main.sh";
      gest = "go clean -testcache && go test -v";

      temp = ''temp_main() { if [ "$1" = "" ]; then echo "one argument required"; return 1; fi; mkdir -p ~/Temp/ && cp -r "$1" ~/Temp/ ; }; temp_main'';
      templs = "ls ~/Temp/";

      runds = "(ds; rm -rf compose/nginx/env.json && make compose-up && make build && make run)";
      v3 = "cd ~/go/src/gitlab.wiserskills.net/wiserskills/v3/";
      ds = "v3 && cd dataserver/";

      gopl = "cd ~/Workspace/playground/go/";
      pl = "playground";
      playground = "cd ~/Workspace/playground/";
      pypl = "cd ~/Workspace/playground/python/";

      projects = "cd ~/Workspace/projects/";
      pr = "projects";

      dot = "cd ~/.dotfiles/";
      down = "cd ~/Downloads/";
      etc = "cd ~/.etc/";
      saves = "cd ~/Saves/";
      tests = "cd ~/Workspace/tests/";
      workspace = "cd ~/Workspace/";

      # etc
      cat = "bat -p";
      clip = "xclip";
      open = "xdg-open";
      po = "poweroff";
      poff = "poweroff";
      xclip = "xclip -selection c";

      neofetch = ''neofetch --ascii "$(fortune | cowsay -W 40)" | lolcat'';
      nfetch = "neofetch";
      ale = "alejandra --quiet";
      dud = "du --human-readable --summarize";
      du = "du --human-readable";
      f = "thefuck";
      fuck = "thefuck";
      gf = "gofumpt";
      k = "k9s";
      ld = "lazydocker";
      lg = "lazygit";
      listen = "ngrok http";
      py = "python3";
      share = "ngrok http";
      tools = "nix-env --query";
      top = "gotop --nvidia";
      unrar = "unar";
      wscat = "websocat";
    };

    sessionVariables = rec {
      CGO_ENABLED = "0";
      GOPATH = "/home/$USER/go";
      GOPRIVATE = "gitlab.wiserskills.net/wiserskills/";
      PATH = "$PATH:$GORROT:$GOPATH/bin";
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

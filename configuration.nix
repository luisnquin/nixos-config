{
  config,
  pkgs,
  ...
}: {
  imports = [
    ./hardware-configuration.nix
  ];

  require = [
    # TODO: modules, units and services must be called once here
    # With this unit the battery will not go higher than 60 percent
    "/etc/nixos/units/battery-limit.nix"
    # Enables nvidia drivers, at least in my computer
    "/etc/nixos/modules/nvidia.nix"
    # Cron tasker without tasks
    "/etc/nixos/services/cron.nix"
    # Some tmux configurations
    "/etc/nixos/modules/tmux.nix"
    # Packages updated/needed
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
    # extraOptions = ''
    #   experimental-features = nix-command flakes
    # '';
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
    motd = "It's a good moment to tell you that this will be a great day for you 🌇";

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

    spotifyd = {
      enable = true;
      settings.global = {
        autoplay = true;
        backend = "pulseaudio";
        bitrate = 320;
        dbus_type = "system";
        device_name = "spotifyd";
        device_type = "computer";
        initial_volume = "80";
        # TODO: implemetation for secrets and other personal settings
        password = "";
        use_keyring = true;
        use_mpris = true;
        username = "yeselony"; # Thanks to the guy who stole and changed my username 5 years ago
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

      # windowManager = {};
    };

    pipewire = {
      # In replacement of pulseaudio that doesn't give a good support for some of my programs
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
          vscode
        ];

        spotify = with pkgs; [
          spotify
          spotifyd
          spotify-tui
        ];

        go = with pkgs; [
          air
          delve
          gcc
          go_1_19
          go-protobuf
          gofumpt
          gopls
          gotools
          grpc-tools
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
          nodePackages.typescript
          nodePackages.pnpm
          nodejs-18_x
          deno
          bun
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
          onefetch
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
          # nodePackages.prettier
          onlyoffice-bin
          obs-studio
          redoc-cli
          websocat
          dbeaver
          postman
          shfmt
          ngrok
          sqlc
          # lens
          tmux
        ];
      };
    in
      # a large etcetera
      [
        # gnome.gnome-sound-recorder
        gnome.seahorse
        stdenv_32bit
        imagemagick
        libsecret
        libnotify
        octofetch
        coreutils
        binutils
        neofetch
        tenacity
        nyancat
        openjdk
        gnumake
        fortune
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
        clang
        wget
        dpkg
        tree
        unar
        gimp
        vim
        vlc
        bat
        zip
        zsh
        jq
      ] # with their rommates
      ++ nyxPkgs.kubernetes
      ++ nyxPkgs.spotify
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
      # Git
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
      ggpull = "git pull origin $(git branch --show-current)";
      gpull = "git pull origin";
      gmpull = "ls | xargs -P10 -I{} git -C {} pull ";
      ggpush = "git push origin $(git branch --show-current)";
      gpush = "git push origin";
      gr = "git reset -q";
      grb = "git rebase";
      gs = "git stash";
      gsp = "git stash pop";
      gss = "git status -s";
      gt = "git tag";

      # Docker
      dka = "docker kill $(docker ps -qa) 2> /dev/null";
      dra = "docker rm $(docker ps -qa) 2> /dev/null";
      dria = "docker rmi -f $(docker image ls -qa)";
      dils = "docker image ls";
      dss = "docker stats";
      dcp = "docker cp";
      dps = "docker ps -a";

      # My own
      nyx = "sh ~/.dotfiles/.scripts/main.sh";
      gest = "go clean -testcache && go test -v";
      setup = "tmux rename-window \"dev-setup 🦭\" \\; split-window -h \\; split-window -v \\; selectp -t 0 \\; split-window -v \\; selectp -t 0 \\; send-keys -t 2 \"nyancat --no-counter\" ENTER \\; send-keys -t 1 \"gotop --nvidia --color=vice\" ENTER \\; send-keys -t 3 \"k9s\" ENTER; clear";

      # It's not like you're not needed
      runds = "(ds; rm -rf compose/nginx/env.json && make compose-up && make build && make run)";
      v3 = "cd ~/go/src/gitlab.wiserskills.net/wiserskills/v3/";
      ds = "v3 && cd dataserver/";

      # Instant tp to some directories
      dot = "cd ~/.dotfiles/";
      down = "cd ~/Downloads/";
      etc = "cd ~/.etc/";
      gopl = "cd ~/Workspace/playground/go/";
      pl = "cd ~/Workspace/playground/";
      pr = "cd ~/Workspace/projects/";
      pypl = "cd ~/Workspace/playground/python/";
      tests = "cd ~/Workspace/tests/";
      workspace = "cd ~/Workspace/";

      # Those who are lazy to write definitely go here
      cat = "bat -p";
      clip = "xclip";
      open = "xdg-open";
      cl = "clear";
      cls = "clear";
      po = "poweroff";
      poff = "poweroff";
      xclip = "xclip -selection c";
      whoseport = "netstat -tulpln 2> /dev/null | grep :";
      neofetch = ''neofetch --ascii "$(fortune | cowsay -W 40)" | lolcat'';
      nyancat = "nyancat --no-counter";
      search = "nix search nixpkgs";
      nfetch = "neofetch";
      ale = "alejandra --quiet";
      dud = "du --human-readable --summarize";
      du = "du --human-readable";
      ld = "lazydocker";
      lg = "lazygit";
      listen = "ngrok http";
      py = "python3";
      share = "ngrok http";
      top = "gotop --nvidia --color=vice";
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

    # Google search, zsh history search, highlighter for conventional
    # branches(include jira tickets) and tmux startup in non-vscode editors
    interactiveShellInit = ''
      google() {
          search=""
          for term in "$@"; do search="$search%20$term"; done
          xdg-open "http://www.google.com/search?q=$search"
      }

      if [[ $(ps -p$$ -ocmd=) == *"zsh"* ]]; then hsi() grep "$*" ~/.zsh_history; fi

      # I'm not wrong leaving this here
      gbh() {
        color_end="\033[0m"
        purple="\033[0;95m"
        yellow="\033[0;93m"
        black="\033[0;90m"
        green="\033[0;92m"
        blue="\033[0;94m"

        is_current_branch=0

        emojis=(🍍 🍝 🥞 🥗 🧋 🍁 🍂 🍃 🌱)

        month_nb=$(date +%m)

        case $month_nb in
        1|2) # Summer
          emojis+=(🐚 🌴 🍹 🌻 🏊 ☀️ 👙)

          # Valentine's month
          if [[ $month_nb -eq 2 ]]; then
            emojis+=(💞 🍫 🧸 💐 🌹 💌)
          fi

          ;;
        3|4|5) # Spring
          emojis+=(🐣 🌳 🍀 🍃 🌈 🌷 🐝 🐇)

          ;;
        9|11) # Autumn
          emojis+=(🍂 🥮 ☕ 🌰 🍊)

          if [[ $month_nb -eq 11 ]]; then
            emojis+=(🎂 🍰  🎁 🎉 🎈)
          fi

          ;;
        10) # Halloween
          emojis+=(🐈‍⬛ 🦇 🕷️ 🥀 🍬 🍫 🎃 🍭 ⚰️ 🪦 🫀)

          ;;
        12) $ Christmas
          emojis+=(🍷 🎁 🎄 ☃️ ❄️ 🥛 🦌)
        esac

        emoji=$emojis[$((RANDOM%$#emojis[@]))]

        for branch in $(git branch | head -n 15); do
            if [[ "$branch" == "*" ]]; then
                is_current_branch=1

                continue
            fi

            frags=($(echo "$branch" | tr "/" "\n"))

            result=""

            if [[ "$branch" == feat/* ]]; then
                result="$green$frags[1]/$color_end$frags[2]"
            elif [[ "$branch" == fix/* ]]; then
                result="$yellow$frags[1]/$color_end$frags[2]"
            elif [[ "$branch" == refactor/* ]]; then
                result="$green$frags[1]/$color_end$frags[2]"
            elif [[ "$branch" == dev/* ]]; then
                result="$purple$frags[1]/$color_end$frags[2]"
            elif [[ "$branch" == chore/* ]]; then
                result="$black$frags[1]/$color_end$frags[2]"
            elif [[ "$branch" == "master" || "$branch" == "main" ]]; then
                result="$blue$branch$color_end"
            else
                result="$branch"
            fi

            if [[ $is_current_branch -eq 1 ]]; then
                if [[ $emoji == "" ]]; then
                  emoji="*"
                fi

                result="$result $emoji"

                is_current_branch=0
            fi

            # Regex for Jira ticketsssss
            if [[ $(echo "$result" | grep -E --colour=auto "[A-Z0-9]{2,}-\d+*") == "" ]]; then
                echo "$result"
            else
                echo "$result" | grep -E --colour=auto "[A-Z0-9]{2,}-\d+*"
            fi
        done
      }

      if [ "$TMUX" = "" ] && [ "$TERM_PROGRAM" != "vscode" ] ; then exec tmux; fi
    '';
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

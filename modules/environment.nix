{
  config,
  pkgs,
  ...
}: {
  environment = {
    systemPackages = with pkgs; let
      # TODO: organize by section with subsections
      nyxPkgs = {
        kubernetes = with pkgs; [
          kubernetes
          minikube
          kubectl
          # lens
          k9s
        ];

        apps = with pkgs; [
          fragments
          discord
          slack
        ];

        browsers = with pkgs; [
          vivaldi
          brave
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
          golangci-lint
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
          nil
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
          virtualenv
          python310
          pyright
        ];

        dev = with pkgs; [
          nodePackages.firebase-tools
          license-generator
          onlyoffice-bin
          obs-studio
          redoc-cli
          websocat
          awscli2
          dbeaver
          postman
          minify
          csvkit
          vscode
          shfmt
          ngrok
          sqlc
          tmux
        ];
      };
    in
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
        exa # ls command replacement
        vlc
        bat
        zip
        zsh
        jq
      ] # with their rommates
      ++ nyxPkgs.kubernetes
      ++ nyxPkgs.browsers
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
      # TODO: check .git
      gmb = ''command ls | xargs -i sh -c 'printf " ~ \033[0;94m{}\033[0m:" && git -C {} branch --show-current' '';
      gmpull = "command ls | xargs -i git -C {} pull";
      gmpush = "command ls | xargs -i git -C {} push";
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
      setup = "tmux rename-window \"setup 🦭\" \\; split-window -h \\; split-window -v \\; resize-pane -D 4 \\; selectp -t 0 \\; split-window -v \\; resize-pane -D 4 \\; selectp -t 0 \\; send-keys -t 2 \"spt\" ENTER \\; send-keys -t 1 \"gotop --nvidia --color=vice\" ENTER \\; send-keys -t 3 \"k9s  --readonly\" ENTER; clear";

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
      tests = "cd ~/Tests/";
      tmp = "cd /tmp/";
      workspace = "cd ~/Workspace/";

      # Overwriten program calls
      cp = "cp --interactive";
      rm = "rm --interactive";
      mv = "mv --interactive";
      du = "du --human-readable";
      xclip = "xclip -selection c";
      ls = "exa --icons";
      ll = "exa -l";
      la = "exa -a";
      cat = "bat -p";

      # Those who are lazy to write definitely go here
      open = "xdg-open";
      cl = "clear";
      cls = "clear";
      po = "poweroff";
      poff = "poweroff";
      whoseport = "netstat -tulpln 2> /dev/null | grep :";
      nyancat = "nyancat --no-counter";
      search = "nix search nixpkgs";
      ale = "alejandra --quiet";
      dud = "du --human-readable --summarize";
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
      # The other related config only apply to the build
      NIXPKGS_ALLOW_UNFREE = "1";
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

      emoji() {
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
          12) # Christmas
              emojis+=(🍷 🎁 🎄 ☃️ ❄️ 🥛 🦌)
          esac

          echo $emojis[((RANDOM%$#emojis[@]))]
      }

      # I'm not wrong leaving this here
      gbh() {
        color_end="\033[0m"
        purple="\033[0;95m"
        yellow="\033[0;93m"
        black="\033[0;90m"
        green="\033[0;92m"
        blue="\033[0;94m"

        is_current_branch=0

        em=$(emoji)

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
                if [[ $em == "" ]]; then
                  em="*"
                fi

                result="$result $em"
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
}

{
  config,
  pkgs,
  ...
}: let
  startupShellUtils = builtins.readFile ./utils.zsh;
in {
  environment = {
    systemPackages = with pkgs; let
      # TODO: organize by section with subsections
      gg = {
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
          gofumpt
          golangci-lint
          gopls
          gotools
          govulncheck
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
          act
          gitlab-runner
          git
          lazygit
          pre-commit
          git-ignore
          git-chglog # I'm using conventional commits so
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

        osint = with pkgs; [
          exiftool
        ];

        dev = with pkgs; [
          nodePackages.firebase-tools
          license-generator
          onlyoffice-bin
          obs-studio
          redoc-cli
          websocat
          awscli2
          # gobang
          dbeaver
          postman
          minify
          csvkit
          vscode
          shfmt
          ngrok
          sqlc
          tmux
          # CLI to run SQL queries agains JSON, CSV, XLSX, etc
          dsq
        ];
      };
    in
      [
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
        genact
        cowsay
        ffmpeg
        ntfs3g
        neovim
        lolcat
        genact
        etcher
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
      ++ gg.kubernetes
      ++ gg.browsers
      ++ gg.spotify
      ++ gg.python
      ++ gg.docker
      ++ gg.osint
      ++ gg.rust
      ++ gg.apps
      ++ gg.dev
      ++ gg.git
      ++ gg.nix
      ++ gg.js
      ++ gg.go;

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
      ggc = "git gc --aggressive";
      gmgc = "command ls | xargs -i git -C {} gc --aggressive";
      gls = "git log --oneline | head -n 10";
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
      setup = "tmux rename-window \"setup 🦭\" \\; split-window -h \\; split-window -v \\; resize-pane -D 3 \\; selectp -t 0 \\; split-window -v \\; resize-pane -D 3 \\; selectp -t 0 \\; send-keys -t 2 \"spt\" ENTER \\; send-keys -t 1 \"gotop --nvidia --color=vice\" ENTER \\; send-keys -t 3 \"k9s  --readonly\" ENTER; clear";

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
      githubci = "act";
      gitlabci = "gitlab-runner";
      open = "xdg-open";
      cl = "clear";
      cls = "clear";
      po = "poweroff";
      poff = "poweroff";
      actl = "act --list";
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
    interactiveShellInit = startupShellUtils;
  };
}
/*
gc() {
      if [ "$(git branch --show-current)" == "main" ] || [ "$(git branch --show-current)" == "master" ]; then
        read -p "Are you sure? " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
          exit 0
        fi
      fi
      echo
      git commit -v
    }
*/


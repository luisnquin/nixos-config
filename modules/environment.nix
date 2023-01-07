{
  config,
  pkgs,
  ...
}: let
  owner = import "/etc/nixos/owner.nix";
in {
  environment = {
    systemPackages = with pkgs; let
      gg = {
        apps = [
          fragments
          discord
          slack
        ];

        browsers = [
          vivaldi
          brave
        ];

        go = [
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

        rust = [
          cargo
          clippy
          rust-analyzer
          rustc
          rustfmt
          rustup
          vscode-extensions.matklad.rust-analyzer
        ];

        js = [
          nodePackages.typescript
          nodePackages.pnpm
          nodejs-18_x
          deno
          bun
        ];

        nix = [
          alejandra
          nixos-option
          rnix-lsp
          vscode-extensions.jnoortheen.nix-ide
        ];

        python = [
          pyright
          python310
          python310Packages.pipx
          virtualenv
        ];

        osint = [
          exiftool
          maigret
        ];

        dev = [
          nodePackages.firebase-tools
          license-generator
          onlyoffice-bin
          obs-studio
          redoc-cli
          # websocat
          awscli2
          # gobang
          # dbeaver
          zathura
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

          # Processors
          htmlq
          jq
          yq
        ];
      };
    in
      [
        gnome.seahorse
        stdenv_32bit
        coreutils
        libsecret
        binutils
        clang

        # imagemagick
        # libnotify

        # fs
        rclone
        ntfs3g

        # Etc
        octofetch
        neofetch
        nyancat
        openjdk
        gnumake
        openssh
        fortune
        lolcat
        cowsay
        genact

        p7zip
        unzip
        zip

        neovim
        etcher
        exfat
        xclip
        wget
        dpkg
        tree
        unar
        gimp
        vim

        vlc

        exa # ls command replacement
        bat

        zsh

        # System monitoring tools
        gotop
        btop
        htop
      ] # with their rommates
      ++ (with pkgs; builtins.concatLists (builtins.attrValues gg));

    shellAliases = {
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
      open = "xdg-open";
      cl = "clear";
      rc = "rclone";
      cls = "clear";
      po = "poweroff";
      poff = "poweroff";
      actl = "act --list";
      whoseport = "netstat -tulpln 2> /dev/null | grep :";
      nyancat = "nyancat --no-counter";
      search = "nix search nixpkgs";
      ale = "alejandra --quiet";
      dud = "du --human-readable --summarize";
      listen = "ngrok http";
      py = "python3";
      share = "ngrok http";
      top = "gotop --nvidia --color=vice";
      unrar = "unar";
      wscat = "websocat";
    };

    sessionVariables = {
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

    # home-manager.users."${owner.username}" = {};

    # Configuration files
    etc = {
      "zathurarc".text = builtins.readFile ../etc/zathurarc;
    };

    # Google search, zsh history search, highlighter for conventional
    # branches(include jira tickets) and tmux startup in non-vscode editors
    interactiveShellInit = builtins.readFile ./shell_startup.zsh;
  };
}

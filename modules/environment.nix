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
          obs-studio
          discord
          vivaldi
          etcher
          brave
          slack
          gimp
        ];

        dev = [
          # Go-related
          air
          delve
          gcc
          go_1_20
          go-protobuf
          gofumpt
          golangci-lint
          gopls
          gotools
          govulncheck
          grpc-tools

          # Rust-related
          cargo
          clippy
          rust-analyzer
          rustc
          rustfmt
          rustup
          vscode-extensions.matklad.rust-analyzer

          # JavaScript-related
          nodePackages.typescript
          nodePackages.pnpm
          nodejs-18_x
          deno
          bun

          # Nix-related
          alejandra
          nixos-option
          rnix-lsp
          vscode-extensions.jnoortheen.nix-ide
          nix-prefetch-git # Tool to get information from remote repository like sha256

          # Python-related
          pyright
          python311
          python310Packages.pipx
          virtualenv

          # Other
          nodePackages.firebase-tools
          license-generator
          onlyoffice-bin
          redoc-cli
          awscli2
          vscode
          clang

          # websocat
          # dbeaver
          # gobang

          minify # HTML, CSS, and JavaScript minifier
          shfmt # Shell code formatter
          zathura # PDF viewer
          sqlc # SQL generator
          # Processors
          csvkit
          htmlq
          dsq
          jq
          yq
          # HTTP
          postman
          ngrok
        ];

        osint = [
          exiftool
          maigret
          whois
        ];

        preferences = [
          rclone # For management in cloud storages
          freshfetch # neofetch replacement
          xclip # Clipboard
          tldr # Alternative to man

          # Fufu stuff
          octofetch
          nyancat
          genact
          tree
        ];

        core = [
          stdenv_32bit
          coreutils
          libsecret
          binutils
          openssh
          gnumake
          openjdk
          neovim
          wget
          vim
          bat
          exa # ls command replacement
          vlc

          # NTFS
          ntfs3g
          exfat

          gnome.seahorse # Keyring

          # System monitoring tools
          gotop
          btop
          htop

          # Compressed files
          p7zip
          unzip
          unar
          zip
        ];
      };
    in (builtins.concatLists (builtins.attrValues gg));

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
      rc = "rclone";
      cls = "clear";
      poff = "poweroff";
      actl = "act --list";
      neofetch = "freshfetch";
      whoseport = "netstat -tulpln 2> /dev/null | grep :";
      nyancat = "nyancat --no-counter";
      ale = "alejandra --quiet";
      dud = "du --human-readable --summarize";
      man = "tldr";
      # Abstraction
      listen = "ngrok http";
      py = "python3";
      share = "ngrok http";
      top = "gotop --nvidia --color=vice";
      unrar = "unar";
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

    # Configuration files
    etc = {
      "zathurarc".text = builtins.readFile ../dots/etc/zathurarc;
    };

    # Google search, zsh history search, highlighter for conventional
    # branches(include jira tickets) and tmux startup in non-vscode editors
    interactiveShellInit = builtins.readFile ../dots/.zshrc;
  };
}

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
          virtualenv
          (python310.withPackages # Best abstraction you've ever seen
            (p:
              with p; [
                openai # Used by codex
                pipx
              ]))

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
          openssl
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

    # Available in admin shell
    extraInit = let
      fetchEnvironmentInfoScript = builtins.readFile ../dots/scripts/fetch-environment-info.sh;
    in ''
      ${fetchEnvironmentInfoScript}
    '';

    localBinInPath = true;
  };

  # Configuration files
  environment.etc = {
    "zathurarc".text = builtins.readFile ../dots/etc/zathurarc;
  };

  home-manager.users."${owner.username}" = {
    xdg.configFile."go/env".text = builtins.readFile ../dots/home/go/env;
  };
}

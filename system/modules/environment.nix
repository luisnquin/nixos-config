{
  config,
  pkgs,
  ...
}: let
  owner = import ../../owner.nix;
in {
  environment = {
    systemPackages = with pkgs; let
      gg = {
        apps = [
          element-desktop
          # obs-studio
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
          richgo
          tinygo

          # Rust-related
          cargo
          clippy
          rust-analyzer
          rustc
          rustfmt
          rustup

          # JavaScript-related
          nodePackages.typescript
          nodePackages.pnpm
          nodejs-18_x
          deno
          bun

          # Nix-related
          alejandra
          rnix-lsp
          nix-prefetch-git

          # Python-related
          (python310.withPackages
            (p:
              with p; [
                pipx
                pip
              ]))
          virtualenv
          pyright

          # Other
          nodePackages.firebase-tools
          license-generator
          onlyoffice-bin
          redoc-cli
          postman
          rclone # Cloud storages in one CLI
          awscli
          ngrok
          clang

          zathura
          minify # HTML, CSS, and JavaScript minifier
          shfmt
          sqlc # SQL generator
          scc

          # Processors
          csvkit
          htmlq
          dsq
          jq
          yq
        ];

        osint =
          #  Open source intelligence
          [
            exiftool
            maigret
            whois
          ];

        etc = [
          freshfetch # neofetch replacement
          xclip
          tldr # Alternative to man
          octofetch
          nyancat
          ranger
          genact
          tree
          love # Game engine
        ];

        core = [
          gnome.seahorse # Keyring
          stdenv_32bit
          coreutils
          libsecret
          binutils
          openssh
          openssl
          gnumake
          openjdk
          facter # To collect and display system facts
          nmap
          wget
          bat
          exa # ls command replacement
          vlc

          # NTFS
          ntfs3g
          exfat

          # System monitoring tools
          gotop
          btop
          htop

          p7zip
          unzip
          unar
          zip
        ];
      };
    in (builtins.concatLists (builtins.attrValues gg));

    # Root shell
    # extraInit = "";

    localBinInPath = true;
  };

  # Configuration files
  environment.etc = {
    "zathurarc".text = builtins.readFile ../../dots/etc/zathurarc;
  };
}

{
  config,
  pkgs,
  ...
}: let
  owner = import ../owner.nix;
in {
  environment = {
    systemPackages = with pkgs; let
      gg = {
        apps = [
          element-desktop
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
          yarn
          bun

          # Nix-related
          alejandra
          nixos-option
          rnix-lsp
          nix-prefetch-git # Tool to get information from remote repository like sha256

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
          # awscli2
          clang

          # websocat
          # dbeaver
          # gobang

          scc #  Lines of code in a directory with complexity estimation.
          # useful useful useful useful useful usefulusefulusefulusefulusefulusefulusefuluseful
          minify # HTML, CSS, and JavaScript minifier
          shfmt # Shell code formatter
          zathura # PDF viewer
          sqlc # SQL generator
          # Processors
          awscli
          csvkit
          htmlq
          dsq
          jq
          yq
          # HTTP
          postman
          ngrok
        ];

        osint =
          #  Open source intelligence
          [
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
          ranger
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
          facter # To collect and display system facts
          nmap
          wget
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
}

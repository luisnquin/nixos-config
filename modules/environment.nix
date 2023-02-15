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
          element-web
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
          pyright
          virtualenv

          # Best abstraction you've ever seen
          (python310.withPackages
            (p:
              with p; [
                pipx
              ]))

          # Other
          nodePackages.firebase-tools
          license-generator
          onlyoffice-bin
          redoc-cli
          awscli2
          clang

          # websocat
          # dbeaver
          # gobang

          scc #  Lines of code in a directory with complexity estimation.
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

  home-manager = with owner; {
    users."${username}" = {
      xdg.configFile = {
        "go/env".text = builtins.readFile ../dots/home/go/env;

        "openaiapirc".text = ''
          [openai]
          organization_id = ${openai.organization-id}
          secret_key = ${openai.secret-key}
        '';

        "ai/.ai-cli".text = ''
          OPENAI_API_KEY=${openai.secret-key}
        '';

        "rclone/rclone.conf".text = builtins.concatStringsSep "\n" (builtins.attrValues rclone);
      };
    };
  };
}

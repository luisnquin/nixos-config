{
  config,
  pkgs,
  ...
}: {
  environment = {
    systemPackages = with pkgs; let
      gg = {
        apps = [
          simplescreenrecorder
          obs-studio

          element-desktop
          obsidian
          discord
          vivaldi
          etcher
          brave
          slack
          gimp
        ];

        dev = [
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
          hyperfine # Benchmarking tool
          commitlint
          asciinema
          redoc-cli
          postman
          gnumake
          argocd
          rclone # Cloud storages in one CLI
          awscli
          ngrok
          clang
          just

          haskellPackages.NanoID # To generate nano id from terminal
          wkhtmltopdf-bin
          zathura
          minify # HTML, CSS, and JavaScript minifier
          shfmt
          sqlc # SQL generator
          scc

          # Processors
          csvkit
          htmlq
          yq-go
          dsq
          jq
        ];

        osint =
          #  Open source intelligence
          [
            exiftool
            maigret
            whois
          ];

        etc = [
          translate-shell # Translate anything from shell
          xclip
          ranger
          tldr # Alternative to man
          tree

          asciiquarium
          freshfetch # neofetch replacement
          octofetch
          nyancat
          cmatrix
          facter
          genact
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
          openjdk
          gparted
          facter # To collect and display system facts
          btop
          nmap
          wget
          bat
          exa # ls command replacement
          vlc

          # NTFS
          ntfs3g
          exfat

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
    "zathurarc".text = builtins.readFile ../dots/etc/zathurarc;
  };
}

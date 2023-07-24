{pkgs, ...}: {
  environment = {
    systemPackages = with pkgs; let
      gg = {
        apps = [
          simplescreenrecorder
          obs-studio

          element-desktop
          discord
          vivaldi
          etcher
          brave
          # Fix login issues in desktop application(KDE):
          # https://stackoverflow.com/questions/70867064/signing-into-slack-desktop-not-working-on-4-23-0-64-bit-ubuntu
          slack
          gimp
        ];

        dev = [
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
          # pgweb
          ngrok
          clang
          just

          haskellPackages.NanoID # To generate nano id from terminal
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
          xsv
          jq
        ];

        osint =
          #  Open source intelligence
          [
            exiftool
            maigret
            whois
          ];

        auditing = [
          osv-scanner
          semgrep
        ];

        etc = [
          translate-shell # Translate anything from shell
          xclip
          ranger
          glow # To render markdown in the terminal
          tldr # Alternative to man
          tree

          asciiquarium
          octofetch
          macchina
          nyancat
          cmatrix
          scrcpy
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
          killall
          facter # To collect and display system facts
          cmake
          btop
          nmap
          wget
          bat
          dig
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
    in
      builtins.concatLists (builtins.attrValues gg);

    # Root shell
    # extraInit = "";

    localBinInPath = true;
  };

  # Configuration files
  environment.etc = {
    "zathurarc".text = builtins.readFile ../dots/etc/zathurarc;
  };
}

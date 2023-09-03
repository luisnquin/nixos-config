{
  tomato-c,
  pkgs,
  senv,
  ...
}: {
  environment = {
    systemPackages = with pkgs; let
      nixpkgs = builtins.concatLists (builtins.attrValues {
        apps = [
          element-desktop
          obs-studio
          discord
          etcher
          slack # https://stackoverflow.com/questions/70867064/signing-into-slack-desktop-not-working-on-4-23-0-64-bit-ubuntu
          gimp
        ];

        clap = [
          translate-shell # Translate anything from shell
          xclip
          ranger
          tldr # Alternative to man
          tree

          macchina
          genact
        ];

        essentials = [
          gnome.seahorse # Keyring
          stdenv_32bit
          coreutils
          libsecret
          binutils
          openssh
          openssl
          gparted
          cmake
          btop
          nmap
          wget
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

        osint = [
          exiftool
          maigret
          whois
        ];
      });
    in
      nixpkgs
      ++ [
        tomato-c
        senv
      ];

    # Root shell
    # extraInit = "";

    localBinInPath = true;
  };

  # Configuration files
  environment.etc = {
    "zathurarc".text = builtins.readFile ../dots/etc/zathurarc;
  };
}

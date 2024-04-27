{
  pkgsx,
  pkgs,
  senv,
  ...
}: {
  environment = {
    systemPackages = with pkgs; let
      nixpkgs = builtins.concatLists (builtins.attrValues {
        apps = [
          slack # Sucks by default on KDE https://stackoverflow.com/questions/70867064/signing-into-slack-desktop-not-working-on-4-23-0-64-bit-ubuntu
          gimp # Image editor but I don't use it
        ];

        clap = [
          translate-shell # Translate anything from shell
          tealdeer # Alternative to tdlr client and man
          # pkgsx.minecraft
          pkgsx.no
          tree

          pkgsx.daktilo
          macchina
          genact
        ];

        essentials = [
          gnome.seahorse # Keyring
          stdenv_32bit
          pkgsx.kmon
          coreutils
          libsecret
          binutils
          openssh
          openssl
          gparted
          cmake
          btop
          lsof
          nmap
          wget
          dig
          eza # ls command replacement
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
          pkgsx.havn
          exiftool
          # maigret # Broken -> ValueError: Hash algorithm not known for ed448 - use .cms_hash_algorithm for CMS purposes. More info at https://github.com/wbond/asn1crypto/pull/230.
          whois
        ];
      });
    in
      nixpkgs
      ++ [
        senv
      ];

    # Root shell
    # extraInit = "";

    localBinInPath = true;
  };
}

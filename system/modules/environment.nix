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
          pkgsx.no
          tree

          pkgsx.daktilo
          macchina
          genact
        ];

        essentials = [
          seahorse # Keyring
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

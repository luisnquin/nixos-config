{
  pkgsx,
  pkgs,
  senv,
  ...
}: {
  environment = {
    systemPackages = with pkgs; let
      nixpkgs = builtins.concatLists (builtins.attrValues {
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

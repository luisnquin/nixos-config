{pkgs, ...}: {
  environment = {
    systemPackages = with pkgs; [
      translate-shell
      stdenv_32bit
      libsecret
      coreutils
      binutils
      tealdeer
      gparted
      openssl
      cmake
      tree
      lsof
      wget
      eza # ls command replacement

      # ntfs
      ntfs3g
      exfat

      p7zip
      unzip
      unar
      zip
    ];

    localBinInPath = true;
  };
}

{pkgs, ...}: {
  environment = {
    systemPackages = with pkgs; [
      stdenv_32bit
      libsecret
      coreutils
      binutils
      gparted
      openssl
      cmake
      tree
      lsof
      wget
      eza # ls command replacement

      bison
      flex

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

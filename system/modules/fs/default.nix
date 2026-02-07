{
  imports = [
    ./incron.nix
    ./utils.nix
  ];

  boot = {
    supportedFilesystems = ["ntfs"];
    tmp.cleanOnBoot = true;
  };

  services = {
    udisks2.enable = true;
    # without it, the SSD's controller will not know where the free space is on the drive. The controller needs
    # free space to do its garbage collection job. unexpected behavior may occur.
    fstrim.enable = true;
  };
}

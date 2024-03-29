{
  boot = {
    supportedFilesystems = ["ntfs"];
    tmp.cleanOnBoot = true;
  };

  services.udisks2.enable = true;
}

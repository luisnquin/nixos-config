{
  imports = [
    ./incron.nix
    ./utils.nix
  ];

  boot = {
    supportedFilesystems = ["ntfs"];
    tmp.cleanOnBoot = true;
  };

  services.udisks2.enable = true;
}

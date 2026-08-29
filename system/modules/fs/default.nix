{
  imports = [
    ./ephemeral.nix
    ./incron.nix
    ./persistence.nix
    ./utils.nix
  ];

  boot = {
    supportedFilesystems = ["ntfs"];
    tmp = {
      # otherwise /tmp shares the 8G tmpfs root and a large build ENOSPCs /
      useTmpfs = true;
      tmpfsSize = "8G";
    };
  };

  services = {
    udisks2.enable = true;
    udev.extraRules = ''
      ACTION=="add|change", KERNEL=="nvme*n[0-9]", ATTR{queue/scheduler}="mq-deadline"
    '';
    # without it, the SSD's controller will not know where the free space is on the drive. The controller needs
    # free space to do its garbage collection job. unexpected behavior may occur.
    fstrim.enable = true;
  };
}

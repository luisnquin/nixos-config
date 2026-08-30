{
  imports = [
    ./ephemeral.nix
    ./incron.nix
    ./persistence.nix
    ./utils.nix
  ];

  boot = {
    supportedFilesystems = ["ntfs"];
    # /tmp is a bind onto the nvme below, so it survives a reboot on its own
    tmp.cleanOnBoot = true;
  };

  fileSystems."/tmp" = {
    device = "/persist/tmpdir";
    fsType = "none";
    options = ["bind"];
    depends = ["/persist"];
  };

  # /tmp needs 1777, which nix rejects as a build-dir ancestor, so this cannot
  # live under the /persist/tmp that holds nix-builds
  systemd.tmpfiles.rules = ["d /persist/tmpdir 1777 root root -"];

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

{
  # Won't be imported anywhere until I have a free disk on my laptop because I also have a Windows partition.
  # https://github.com/nix-community/disko/blob/master/docs/quickstart.md#quickstart-guide
  #
  # Code below is just copy and it will need to be updated. Check `lsblk`.
  disko.devices = {
    disk = {
      nvme0n1 = {
        device = "/dev/nvme0n1";
      };

      nvme0n6 = {
        device = "/dev/nvme0n6";
        type = "disk";
        content = {
          type = "gpt";
          partitions = {
            ESP = {
              type = "EF00"; # EFI partition type.
              size = "500M";
              content = {
                type = "filesystem";
                format = "vfat";
                mountpoint = "/boot";
              };
            };
            root = {
              start = "901G"; # Start immediately after Windows partition.
              size = "100%"; # Takes the remaining half of the disk space.
              content = {
                type = "filesystem";
                format = "ext4";
                mountpoint = "/";
              };
            };
          };
        };
      };
    };
  };
}

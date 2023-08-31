{
  fallout-grub-theme,
  host,
  pkgs,
  ...
}: {
  boot = with pkgs; {
    loader = {
      efi.canTouchEfiVariables = true;
      timeout = 15;

      grub = {
        gfxmodeBios = host.resolution;
        gfxmodeEfi = host.resolution;
        theme = fallout-grub-theme;

        enable = true;
        device = "nodev";
        useOSProber = true;
        efiSupport = true;
        forceInstall = false;
      };
    };

    kernelPackages = linuxPackages_latest;
    supportedFilesystems = ["ntfs"];
    tmp.cleanOnBoot = true;
  };
}
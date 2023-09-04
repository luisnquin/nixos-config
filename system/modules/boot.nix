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
        enable = true;
        device = "nodev";

        forceInstall = false;
        useOSProber = true;

        # For a better future: https://github.com/NixOS/nixpkgs/issues/23926
        configurationLimit = 42;
        efiSupport = true;

        gfxmodeBios = host.resolution;
        gfxmodeEfi = host.resolution;

        splashImage = ./../dots/boot/grub/splash-image.png;
        theme = fallout-grub-theme;
      };
    };

    kernelPackages = linuxPackages_latest;
    supportedFilesystems = ["ntfs"];
    tmp.cleanOnBoot = true;
  };
}

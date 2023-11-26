{
  fallout-grub-theme,
  host,
  pkgs,
  lib,
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

    kernelParams = [
      "nvidia_drm.modeset=1"
      "i8042.reset=1"
    ];
    kernelPackages = linuxPackages_latest;
    extraModprobeConfig = ''
      options snd-intel-dspcfg dsp_driver=1
    '';
    supportedFilesystems = ["ntfs"];
    tmp.cleanOnBoot = true;
  };
}

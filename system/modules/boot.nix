{
  grub-pkgs,
  host,
  pkgs,
  ...
}: {
  boot = with pkgs; {
    loader = {
      efi = {
        canTouchEfiVariables = true;
        efiSysMountPoint = "/boot/efi";
      };
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
        theme = grub-pkgs.steins-gate;
      };
    };

    kernelParams = ["i8042.reset=1"];
    kernelPackages = linuxPackages_latest;
    extraModprobeConfig = ''
      options snd-intel-dspcfg dsp_driver=1
    '';
    supportedFilesystems = ["ntfs"];
    tmp.cleanOnBoot = true;
  };
}

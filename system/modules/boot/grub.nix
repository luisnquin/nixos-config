{
  pkgs,
  host,
  ...
}: {
  boot.loader.grub = {
    enable = true;
    device = "nodev";

    forceInstall = false;

    # with an ephemeral root, os-prober no longer excludes the disk holding /
    # and detects /persist as a second NixOS, duplicating every generation
    useOSProber = false;

    # For a better future: https://github.com/NixOS/nixpkgs/issues/23926
    configurationLimit = 42;
    efiSupport = true;
    efiInstallAsRemovable = true;

    gfxmodeBios = host.resolution;
    gfxmodeEfi = host.resolution;

    splashImage = ./dots/splash-image.png;
    theme = pkgs.fallout-grub-theme;
  };
}

{
  inputs,
  system,
  host,
  ...
}: {
  boot.loader.grub = {
    enable = true;
    device = "nodev";

    forceInstall = false;
    useOSProber = true;

    # For a better future: https://github.com/NixOS/nixpkgs/issues/23926
    configurationLimit = 42;
    efiSupport = true;
    efiInstallAsRemovable = true;

    gfxmodeBios = host.resolution;
    gfxmodeEfi = host.resolution;

    splashImage = ./dots/splash-image.png;
    theme = let
      grub-pkgs = inputs.grub-themes.packages.${system};
    in
      grub-pkgs.fallout;
  };
}

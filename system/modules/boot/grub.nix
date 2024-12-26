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


  # This is a hack to make the grub menu appear on the boot screen.
  # I won't needed to do this if I shouldn't deleted the /boot directory before. :D
    extraEntries = let
      # sudo blkid /dev/nvme0n1p3
      windowsBootPartitionUuid = "F2646C4C646C159F";
    in ''
      menuentry 'Windows Boot Manager' --class windows --class os {
        insmod part_gpt
        insmod fat
        search --no-floppy --fs-uuid --set=root ${windowsBootPartitionUuid}
        chainloader /EFI/Microsoft/Boot/bootmgfw.efi
      }
    '';

    gfxmodeBios = host.resolution;
    gfxmodeEfi = host.resolution;

    splashImage = ./dots/splash-image.png;
    theme = let
      grub-pkgs = inputs.grub-themes.packages.${system};
    in
      grub-pkgs.fallout;
  };
}

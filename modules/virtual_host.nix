{
  config,
  pkgs,
  ...
}: {
  # ref: https://search.nixos.org/options?channel=22.11&from=0&size=50&sort=relevance&type=packages&query=virtualization.libvirtd
  virtualization.libvirtd = {
    enable = true;
    onBoot = "start"; # Other value: ignore
    onShutdown = "suspend"; # Other value: shutdown

    qemu.runAsRoot = true;
  };

  environment.systemPackages = with pkgs; [
    virt-manager
    # CPU emulator
    qemu
  ];
}

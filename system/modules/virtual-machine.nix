{
  config,
  pkgs,
  ...
}: {
  # ref: https://search.nixos.org/options?channel=22.11&from=0&size=50&sort=relevance&type=packages&query=virtualization.libvirtd
  virtualisation = {
    libvirtd = {
      enable = true;
      onBoot = "start"; # Other value: ignore
      onShutdown = "suspend"; # Other value: shutdown

      qemu.runAsRoot = true;
    };

    virtualbox.host.enable = true;
  };

  environment.systemPackages = with pkgs; [
    vagrant # Development environments
    virt-manager
    # CPU emulator
    qemu
  ];
}

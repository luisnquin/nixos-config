{pkgs, ...}: {
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
    virt-manager
    vagrant
    qemu
  ];
}

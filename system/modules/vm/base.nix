{
  pkgs,
  user,
  ...
}: {
  virtualisation = {
    libvirtd = {
      enable = true;
      onBoot = "start"; # Other value: ignore
      onShutdown = "suspend"; # Other value: shutdown

      qemu.runAsRoot = true;
    };

    virtualbox.host.enable = true;
  };

  users.${user.alias}.extraGroups = [
    "kvm"
  ];

  environment.systemPackages = with pkgs; [
    virt-manager
    vagrant
    qemu
  ];
}

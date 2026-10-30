{
  zramSwap.enable = true;

  boot.kernel.sysctl = {
    "vm.swappiness" = 180;
    "vm.page-cluster" = 0;
  };

  systemd.oomd.enableUserSlices = true;
}

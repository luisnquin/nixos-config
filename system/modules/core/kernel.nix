{pkgs, ...}: {
  boot = {
    kernelParams = ["i8042.reset=1"];
    kernelPackages = pkgs.linuxPackages;
    kernel.sysctl = {
      "vm.dirty_background_bytes" = 268435456;
      "vm.dirty_bytes" = 2147483648;
      "vm.dirty_writeback_centisecs" = 100;
      "vm.dirty_expire_centisecs" = 1000;
    };
    extraModprobeConfig = ''
      options snd-intel-dspcfg dsp_driver=1
    '';
  };

  security.protectKernelImage = true;
}

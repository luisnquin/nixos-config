{pkgs, ...}: let
  kernel = pkgs.cachyosKernels.linux-cachyos-latest.override {
    cpusched = "bore";
    processorOpt = "x86_64-v3";
    hzTicks = "1000";
    bbr3 = true;
    hardened = false;
  };
in {
  boot = {
    kernelParams = ["i8042.reset=1"];
    kernelPackages = pkgs.linuxKernel.packagesFor kernel;
    extraModprobeConfig = ''
      options snd-intel-dspcfg dsp_driver=1
    '';
  };

  security.protectKernelImage = true;
}

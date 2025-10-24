{pkgs, ...}: {
  boot = {
    kernelParams = ["i8042.reset=1"];
    kernelPackages = pkgs.linuxPackages; # stablevariant?
    extraModprobeConfig = ''
      options snd-intel-dspcfg dsp_driver=1
    '';
  };

  security.protectKernelImage = true;
}

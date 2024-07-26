{
  pkgsx,
  pkgs,
  ...
}: {
  # TODO: must have its own module directory or file but not here
  boot = {
    kernelParams = ["i8042.reset=1"];
    kernelPackages = pkgs.linuxPackages_latest;
    extraModprobeConfig = ''
      options snd-intel-dspcfg dsp_driver=1
    '';
  };

  environment.systemPackages = [
    pkgsx.kmon
  ];
}

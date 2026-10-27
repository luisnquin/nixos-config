{
  config,
  pkgs,
  ...
}: {
  # https://nixos.wiki/wiki/Nvidia"
  services.xserver.videoDrivers = ["nvidia"];
  services.switcherooControl.enable = true;

  # boot.blacklistedKernelModules = ["nouveau"];
  # boot.initrd.kernelModules = ["nvidia" "nvidia_modeset" "nvidia-uvm" "nvidia_drm" "kvm-intel"];

  hardware = {
    # renamed from `hardware.opengl`
    graphics = {
      enable = true;
      enable32Bit = true;
      extraPackages = [pkgs.intel-media-driver];
    };

    nvidia = {
      modesetting.enable = true;
      # nvidia open kernel modules, not nouveau
      open = false;
      nvidiaSettings = true; # `$ nvidia-settings`

      powerManagement = {
        enable = true;
        finegrained = true;
      };

      prime = {
        allowExternalGpu = false;

        # Values below differ from laptop to laptop
        nvidiaBusId = "PCI:1:0:0";
        intelBusId = "PCI:0:2:0";

        # recommended by nvidia
        offload = {
          enable = true;
          enableOffloadCmd = true;
        };
      };

      package = let
        inherit (config.boot.kernelPackages) nvidiaPackages;
      in
        nvidiaPackages.production;
    };
  };
}

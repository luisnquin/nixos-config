{
  config,
  pkgs,
  ...
}: let
  nvidia-offload = pkgs.writeShellScriptBin "nvidia-offload" ''
    export __NV_PRIME_RENDER_OFFLOAD=1
    export __NV_PRIME_RENDER_OFFLOAD_PROVIDER=NVIDIA-G0
    export __GLX_VENDOR_LIBRARY_NAME=nvidia
    export __VK_LAYER_NV_optimus=NVIDIA_only
    exec "$@"
  '';
in {
  # https://nixos.wiki/wiki/Nvidia"
  environment = {
    systemPackages = [
      nvidia-offload
      pkgs.nvtop
    ];

    sessionVariables = {
      NIXOS_OZONE_WL = "1";
      WLR_NO_HARDWARE_CURSORS = "1";
      LIBVA_DRIVER_NAME = "nvidia";
      GBM_BACKEND = "nvidia-drm";
      __GLX_VENDOR_LIBRARY_NAME = "nvidia";
    };
  };

  services.xserver.videoDrivers = ["nvidia"];

  # boot.blacklistedKernelModules = ["nouveau"];
  # boot.initrd.kernelModules = ["nvidia" "nvidia_modeset" "nvidia-uvm" "nvidia_drm" "kvm-intel"];

  hardware = {
    nvidia = {
      modesetting.enable = true;
      # open source (nouveau)
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

        offload = {
          enable = true;
          enableOffloadCmd = true;
        };
      };

      package = config.boot.kernelPackages.nvidiaPackages.beta;
    };

    opengl = {
      enable = true;
      driSupport = true;
      driSupport32Bit = true;
    };
  };
}

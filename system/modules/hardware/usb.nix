{pkgs, ...}: {
  environment.systemPackages = with pkgs; [
    f3 # sudo f3probe --time-ops /dev/sd**
    usbutils
  ];
}

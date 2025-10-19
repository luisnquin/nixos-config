{pkgs, ...}: {
  home.packages = with pkgs; [
    arduino-ide
    mpremote
    picocom
  ];
}

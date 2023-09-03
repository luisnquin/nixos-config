{host, ...}: {
  programs.dconf.enable = true;

  services.xserver = {
    enable = true;
    autorun = true;
    layout = host.keyboardLayout;
    libinput.enable = true;

    displayManager.gdm = {
      enable = true;
      autoSuspend = false;
    };

    desktopManager.xterm.enable = true;
  };
}

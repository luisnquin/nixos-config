{isWayland, ...}: {
  programs.discord = {
    enable = true;
    withOpenASAR = true;
    withVencord = true;
    waylandSupport = isWayland;
  };
}

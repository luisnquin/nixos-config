{isWayland, ...}: {
  # A shitty place to stay
  programs.discord = {
    enable = true;
    withOpenASAR = true;
    withVencord = true;
    waylandSupport = isWayland;
  };
}

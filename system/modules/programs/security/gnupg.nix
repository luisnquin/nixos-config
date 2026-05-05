{pkgs, ...}: {
  programs.gnupg.agent = {
    enable = true;
    enableSSHSupport = true;
    pinentryPackage = pkgs.pinentry-curses;
    settings = {
      # this is your vector attack
      default-cache-ttl = 60 * 30;
      max-cache-ttl = 60 * 40;
    };
  };
}

{pkgs, ...}: {
  programs.gnupg.agent = {
    enable = true;
    enableSSHSupport = true;
    pinentryPackage = pkgs.pinentry-curses;
    settings = {
      # this is your vector attack
      default-cache-ttl = 60 * 15;
      max-cache-ttl = 60 * 20;
    };
  };
}

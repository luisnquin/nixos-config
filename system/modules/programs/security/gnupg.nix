{
  programs.gnupg.agent = {
    enable = true;
    enableSSHSupport = true;
    settings = {
      # this is your vector attack
      default-cache-ttl = 60 * 15;
      max-cache-ttl = 60 * 20;
    };
  };
}

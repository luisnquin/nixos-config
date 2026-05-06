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

  environment.interactiveShellInit = ''
    gpg_unlock() {
      export GPG_TTY="$(tty)"
      gpg-connect-agent updatestartuptty /bye >/dev/null
      printf 'test' | gpg --clearsign >/dev/null
    }
  '';
}

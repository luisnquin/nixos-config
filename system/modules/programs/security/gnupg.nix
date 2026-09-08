{
  lib,
  pkgs,
  ...
}: {
  programs.gnupg.agent = {
    enable = true;
    enableSSHSupport = true;

    settings = {
      default-cache-ttl = 60 * 30;
      max-cache-ttl = 60 * 40;
    };
  };

  # Every passphrase prompt is a pinentry-gate request: a fullscreen terminal window
  # on the graphical session or the reserved console otherwise, plus every
  # terminal that marked its pty (Spectacle). First decision wins.
  programs.pinentry-gate = {
    enable = true;
    user = "luisnquin";
    terminal = [
      (lib.getExe pkgs.ghostty)
      "--class=ghostty.pinentry-gate"
      "--fullscreen=true"
      "--confirm-close-surface=false"
      "-e"
    ];
  };

  environment.interactiveShellInit = ''
    if [ -t 0 ]; then
      export GPG_TTY="$(tty)"
      gpg-connect-agent updatestartuptty /bye >/dev/null 2>&1 || true
    fi

    gpg_unlock() {
      printf 'test' | gpg --clearsign >/dev/null
    }

    gpg_forget() {
      gpgconf --kill gpg-agent
    }
  '';
}

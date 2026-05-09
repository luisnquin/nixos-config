{pkgs, ...}: {
  programs.gnupg.agent = {
    enable = true;
    enableSSHSupport = true;

    pinentryPackage = pkgs.writeShellApplication {
      name = "pinentry-auto";
      runtimeInputs = with pkgs; [
        pinentry-gnome3
        pinentry-curses
      ];

      text = ''
        case "''${PINENTRY_USER_DATA:-gui}" in
          curses) exec pinentry-curses "$@" ;;
          *)      exec pinentry-gnome3 "$@" ;;
        esac
      '';
    };

    settings = {
      default-cache-ttl = 60 * 30;
      max-cache-ttl = 60 * 40;
    };
  };

  environment = {
    variables = {
      PINENTRY_USER_DATA = "gui";
    };

    interactiveShellInit = ''
      if [ -t 0 ]; then
        export GPG_TTY="$(tty)"
        export PINENTRY_USER_DATA=curses
        gpg-connect-agent updatestartuptty /bye >/dev/null 2>&1 || true
      fi

      gpg_unlock() {
        export PINENTRY_USER_DATA=curses
        export GPG_TTY="$(tty)"
        gpg-connect-agent updatestartuptty /bye >/dev/null
        printf 'test' | gpg --clearsign >/dev/null
      }

      gpg_forget() {
        gpgconf --kill gpg-agent
      }
    '';
  };
}

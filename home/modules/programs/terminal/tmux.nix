{
  lib,
  pkgs,
  ...
}: {
  # Local interactive shells exec into tmux instead of the terminal launching it,
  # so tmux inherits TMUX_TMPDIR from the already-sourced hm-session-vars and its
  # socket matches shell/SSH (secureSocket's $XDG_RUNTIME_DIR) — one server, not
  # a stray /tmp one. Exec early so the outer zsh skips full init before it is
  # replaced. Guarded against nested tmux, herdr panes, and SSH (owned by the
  # gateway). black-terminal's own autoStart stays off: it lacks these guards.
  programs.zsh.initContent = lib.mkOrder 550 ''
    if [[ -o interactive && -z "$TMUX" && -z "$HERDR_ENV" && -z "$SSH_CONNECTION" && "$TERM_PROGRAM" != "vscode" && "$USER" != "root" ]]; then
      exec ${lib.getExe pkgs.tmux}
    fi
  '';

  programs.tmux.extraConfig = ''
    set -g allow-passthrough on
  '';
}

{
  lib,
  pkgs,
  ...
}: {
  imports = [
    ./ghostty.nix
    ./herdr.nix
    ./ssh-gateway.nix
  ];

  shared.alacritty.enable = true;

  home.packages = [pkgs.tmuxp];

  # Every ghostty surface is its own pty and so gets its own tmux client. Bare
  # `tmux` is `new-session`, which mints a fresh session per surface and never
  # reclaims the ones left detached when a surface closes. Take over an
  # unattached session when one exists so sessions stop piling up and work
  # survives a ghostty restart. Exec early so the outer zsh skips full init
  # before it is replaced; guarded against herdr panes and SSH (owned by the
  # gateway).
  programs.zsh.initContent = lib.mkOrder 510 ''
    if [[ -z "$TMUX" && -z "''${HERDR_ENV:-}" && -z "''${SSH_CONNECTION:-}" && "$TERM_PROGRAM" != "vscode" && "$USER" != "root" ]]; then
      orphan=$(${lib.getExe pkgs.tmux} list-sessions -f '#{==:#{session_attached},0}' -F '#{session_name}' 2>/dev/null | head -1)
      if [[ -n "$orphan" ]]; then
        exec ${lib.getExe pkgs.tmux} attach-session -t "$orphan"
      else
        exec ${lib.getExe pkgs.tmux} new-session
      fi
    fi
  '';

  programs.tmux.extraConfig = ''
    set -g allow-passthrough on
  '';
}

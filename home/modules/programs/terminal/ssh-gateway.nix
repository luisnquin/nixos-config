{
  lib,
  pkgs,
  ...
}: {
  programs.zsh.initContent = lib.mkBefore ''
    if [[ -o interactive && -n "$SSH_CONNECTION" && -z "$HERDR_ENV" && -z "$TMUX" ]]; then
      print -P ""
      print -P "  incoming ssh — %F{cyan}[h]%f herdr  %F{green}[t]%f tmux  %F{yellow}[z]%f zsh"
      print -Pn "  > "
      read -k 1 -t 10 reply || reply=h
      print ""
      case "$reply" in
        t)
          base=$(${lib.getExe pkgs.tmux} list-sessions -F '#{session_name}' 2>/dev/null | head -1)
          if [[ -n "$base" ]]; then
            exec ${lib.getExe pkgs.tmux} new-session -t "$base" \; set-option destroy-unattached on
          else
            exec ${lib.getExe pkgs.tmux} new-session -A -s ssh
          fi
          ;;
        z) ;;
        *) exec ${lib.getExe pkgs.herdr} --session hyprland ;;
      esac
    fi
  '';
}

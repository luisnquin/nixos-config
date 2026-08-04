{
  lib,
  pkgs,
  ...
}: let
  herdrSession = "hyprland";

  summary = pkgs.writeShellApplication {
    name = "ssh-gateway-summary";
    runtimeInputs = with pkgs; [coreutils herdr jq systemd tmux];
    text = builtins.readFile ./ssh-gateway-summary.sh;
  };
in {
  programs.zsh.initContent = lib.mkBefore ''
    if [[ -o interactive && -t 0 && -n "$SSH_CONNECTION" && -z "$HERDR_ENV" && -z "$TMUX" ]]; then
      gw_shown=""
      gw_lines=0
      while true; do
        gw_summary=$(${lib.getExe summary} ${herdrSession} 2>/dev/null)
        if [[ -z "$gw_summary" ]]; then
          gw_summary=$'\n  incoming ssh — [h] herdr  [t] tmux  [z] zsh'
        fi
        if [[ "$gw_summary" != "$gw_shown" ]]; then
          (( gw_lines > 0 )) && print -n $'\e['"$gw_lines"$'A\e[J'
          print -r -- "$gw_summary"
          print -Pn "  > "
          # array assignment is the only form that keeps the blank separator
          # lines; a scalar ''${#''${(@f)...}} silently drops them and the
          # cursor walks up into the banner on every repaint
          gw_rows=("''${(@f)gw_summary}")
          gw_lines=$#gw_rows
          gw_shown="$gw_summary"
        fi
        read -k 1 -t 2 reply && break
      done
      unset gw_summary gw_shown gw_lines gw_rows
      print ""
      case "$reply" in
        h) exec ${lib.getExe pkgs.herdr} --session ${herdrSession} ;;
        t)
          base=$(${lib.getExe pkgs.tmux} list-sessions -F '#{session_last_attached} #{session_name}' 2>/dev/null | sort -rn | head -1 | cut -d' ' -f2-)
          if [[ -n "$base" ]]; then
            exec ${lib.getExe pkgs.tmux} new-session -t "$base" \; set-option destroy-unattached on
          else
            exec ${lib.getExe pkgs.tmux} new-session -A -s ssh
          fi
          ;;
        *) ;;
      esac
    fi
  '';
}

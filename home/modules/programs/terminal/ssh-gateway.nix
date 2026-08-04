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
    if [[ -o interactive && -n "$SSH_CONNECTION" && -z "$HERDR_ENV" && -z "$TMUX" ]]; then
      gw_summary=$(${lib.getExe summary} ${herdrSession} 2>/dev/null)
      if [[ -n "$gw_summary" ]]; then
        print -r -- "$gw_summary"
      else
        print -P ""
        print -P "  incoming ssh — %F{cyan}[h]%f herdr  %F{green}[t]%f tmux  %F{yellow}[z]%f zsh"
      fi
      unset gw_summary
      print -Pn "  > "
      read -k 1 -t 10 reply || reply=h
      print ""
      case "$reply" in
        t)
          base=$(${lib.getExe pkgs.tmux} list-sessions -F '#{session_last_attached} #{session_name}' 2>/dev/null | sort -rn | head -1 | cut -d' ' -f2-)
          if [[ -n "$base" ]]; then
            exec ${lib.getExe pkgs.tmux} new-session -t "$base" \; set-option destroy-unattached on
          else
            exec ${lib.getExe pkgs.tmux} new-session -A -s ssh
          fi
          ;;
        z) ;;
        *) exec ${lib.getExe pkgs.herdr} --session ${herdrSession} ;;
      esac
    fi
  '';
}

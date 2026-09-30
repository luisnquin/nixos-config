#!/usr/bin/env zsh

agent-picker() {
  emulate -L zsh

  local -a candidates=(
    'claude --dangerously-skip-permissions'
    'codex'
    'freebuff'
    'agent'
    'opencode'
    'grok'
    'hermes'
    'pi'
  )

  local -a available=()
  local candidate
  for candidate in "${candidates[@]}"; do
    (( $+commands[${candidate%% *}] )) && available+=("$candidate")
  done

  local selected
  selected="$(print -rl -- "${available[@]}" | fzf --prompt="agent > " --height=40% --layout=reverse)"

  if [[ -z "$selected" ]]; then
    zle reset-prompt
    return 0
  fi

  BUFFER="$selected"
  zle accept-line
}

zle -N agent-picker
bindkey '^A' agent-picker

#!/usr/bin/env zsh

typeset -gA _agent_dir_flag=(
  claude --add-dir
  codex --add-dir
  agent --add-dir
  grok --add-dir
)

_agent_ws_file() {
  print -r -- "${XDG_STATE_HOME:-${HOME%/}/.local/state}/agent-picker/workspaces"
}

_agent_ws_record() {
  local file=$(_agent_ws_file) entry=${(pj:\t:)@}
  local -a kept=()
  [[ -r $file ]] && kept=(${(f)"$(<$file)"})
  kept=("${(@)kept:#$entry}" "$entry")
  (( $#kept > 100 )) && shift $(( $#kept - 100 )) kept
  mkdir -p -- ${file:h} && print -rl -- "${kept[@]}" >| $file.$$ && mv -f -- $file.$$ $file
}

_agent_ws_recent() {
  local file=$(_agent_ws_file) line dir
  [[ -r $file ]] || return 0
  local -a lines=(${(f)"$(<$file)"}) dirs
  for line in "${(@Oa)lines}"; do
    [[ $line == $PWD ]] && continue
    dirs=(${(ps:\t:)line})
    for dir in $dirs; do [[ -d $dir ]] || continue 2; done
    print -r -- "${(j:  +  :)${(@Q)${(@D)dirs}}}"$'\t'"$line"
  done
}

_agent_ws_add() {
  local out rc dir
  out="$( { (( $+commands[zoxide] )) && zoxide query -l } | grep -vxF -- "$PWD" |
    fzf --multi --print-query --expect=alt-enter --prompt="add dirs > " \
      --header="tab: mark  enter: add marked  alt-enter: add typed path" \
      --height=40% --layout=reverse)"
  rc=$?
  (( rc == 0 || rc == 1 )) || return 1
  local -a lines=("${(@f)out}") picked=()
  if [[ $lines[2] == alt-enter ]]; then
    lines=("${lines[1]/#\~/${HOME%/}}")
  else
    lines=("${(@)lines[3,-1]}")
  fi
  for dir in $lines; do
    [[ -d $dir && $dir != *$'\t'* ]] && picked+=(${dir:A})
  done
  (( $#picked )) && print -rl -- $picked
}

agent-picker() {
  emulate -L zsh

  local -a candidates=(
    'claude'
    'pi'
    'codex'
    'freebuff'
    'agent'
    'opencode'
    'grok'
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

  local flag=${_agent_dir_flag[$selected]}
  if [[ -z $flag ]]; then
    BUFFER="$selected"
    zle accept-line
    return 0
  fi

  local choice
  choice="$( {
    print -r -- "${(Q)${(D)PWD}}"$'\t'"$PWD"
    _agent_ws_recent
    print -r -- $'+ add folders\t+'
  } | fzf --delimiter=$'\t' --with-nth=1 --prompt="$selected workspace > " --height=40% --layout=reverse)"

  local -a dirs=(${(ps:\t:)choice})
  shift dirs 2>/dev/null
  if [[ $dirs[1] == + ]]; then
    local added
    added="$(_agent_ws_add)" && dirs=($PWD ${(f)added}) || dirs=()
  fi

  if (( ! $#dirs )); then
    zle reset-prompt
    return 0
  fi

  dirs=(${(u)dirs})
  _agent_ws_record $dirs

  local cmd=$selected dir
  for dir in ${dirs[2,-1]}; do
    cmd+=" $flag ${(q-)dir}"
  done
  [[ $dirs[1] != $PWD ]] && cmd="cd ${(q-)dirs[1]} && $cmd"

  BUFFER="$cmd"
  zle accept-line
}

zle -N agent-picker
bindkey '^A' agent-picker

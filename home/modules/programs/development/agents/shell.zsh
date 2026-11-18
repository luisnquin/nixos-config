#!/usr/bin/env zsh

typeset -gA _agent_dir_flag=(
  claude --add-dir
  codex --add-dir
  agent --add-dir
  grok --add-dir
)

_agent_tilde() {
  local home=${HOME%/}
  case $1 in
    $home) REPLY='~' ;;
    $home/*) REPLY="~/${1#$home/}" ;;
    *) REPLY=$1 ;;
  esac
}

_agent_ws_file() {
  print -r -- "${XDG_STATE_HOME:-${HOME%/}/.local/state}/agent-picker/workspaces"
}

_agent_ws_extras() {
  local file=$(_agent_ws_file) line dir
  reply=()
  [[ -r $file ]] || return 0
  for line in ${(f)"$(<$file)"}; do
    [[ ${line%%$'\t'*} == $1 ]] || continue
    for dir in ${${(ps:\t:)line}[2,-1]}; do
      [[ -d $dir ]] && reply+=($dir)
    done
  done
}

_agent_ws_record() {
  local file=$(_agent_ws_file) line key
  local -a kept=()
  [[ -r $file ]] && for line in ${(f)"$(<$file)"}; do
    key=${line%%$'\t'*}
    [[ $key != $1 && -d $key ]] && kept+=($line)
  done
  (( $# > 1 )) && kept+=(${(pj:\t:)@})
  (( $#kept > 100 )) && shift $(( $#kept - 100 )) kept
  mkdir -p -- ${file:h} && print -rn -- ${(F)kept}${kept:+$'\n'} >| $file.$$ && mv -f -- $file.$$ $file
}

_agent_ws_edit() {
  local here=$1 out dir pos=1
  local -a lines fields extras
  while true; do
    _agent_ws_extras $here
    extras=($reply)
    out="$( {
      for dir in $extras; do _agent_tilde $dir; print -r -- "$REPLY  (in workspace, del removes)"$'\t'"$dir"$'\tin'; done
      (( $+commands[zoxide] )) && zoxide query -l | grep -vxFf <(print -rl -- $here $extras) |
        while IFS= read -r dir; do _agent_tilde $dir; print -r -- "$REPLY"$'\t'"$dir"$'\tnew'; done
    } | winnow --print-query --expect=del --delimiter=$'\t' --with-nth=1 --cursor=$pos \
        --prompt="add to workspace > " \
        --header="enter: add  ~/path + enter: add typed path  del: remove  esc: back")" || (( $? == 1 )) || return 0

    lines=("${(@f)out}")
    fields=(${(ps:\t:)lines[3]})
    dir=$fields[2]
    if [[ $lines[2] == del ]]; then
      [[ $fields[3] == in ]] || continue
      pos=${extras[(i)$dir]}
      _agent_ws_record $here ${extras:#$dir}
      continue
    fi
    if [[ $lines[1] == [/~]* ]]; then
      dir=${lines[1]/#\~/${HOME%/}}
      [[ -d $dir && $dir != *$'\t'* ]] || return 0
      dir=${dir:A}
    elif [[ $fields[3] != new ]]; then
      return 0
    fi
    extras+=($dir)
    [[ $dir == $here ]] || _agent_ws_record $here ${(u)extras}
    return 0
  done
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
  selected="$(print -rl -- "${available[@]}" | winnow --prompt="agent > ")"

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

  local here=${PWD:A} dir choice pos=1 nl=$'\n'
  local -a extras shown
  while true; do
    _agent_ws_extras $here
    extras=($reply)
    (( $#extras )) || pos=1

    choice="$( {
      _agent_tilde $here
      print -r -- "$REPLY (current dir)"$'\tcur'
      if (( $#extras )); then
        shown=()
        for dir in $here $extras; do _agent_tilde $dir; shown+=($REPLY); done
        print -r -- "${(j:  +  :)shown}"$'\tws'
      fi
      print -r -- $'+ add to workspace\tadd'
    } | winnow --delimiter=$'\t' --with-nth=1 --prompt="$selected > " --cursor=$pos \
        --header="enter: select  esc: cancel${extras:+$nl+ add to workspace: enter adds a folder, del removes one}")"
    pos=1

    case ${choice##*$'\t'} in
      cur) extras=(); break ;;
      ws) _agent_ws_record $here $extras; break ;;
      add) _agent_ws_edit $here; pos=2 ;;
      *) zle reset-prompt; return 0 ;;
    esac
  done

  local cmd=$selected
  for dir in $extras; do
    _agent_tilde $dir
    case $REPLY in
      '~') cmd+=" $flag ~" ;;
      '~/'*) cmd+=" $flag ~/${(q-)REPLY#\~/}" ;;
      *) cmd+=" $flag ${(q-)REPLY}" ;;
    esac
  done

  BUFFER="$cmd"
  zle accept-line
}

zle -N agent-picker
bindkey '^A' agent-picker

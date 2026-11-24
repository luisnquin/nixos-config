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

_agent_short() {
  _agent_tilde $1
  REPLY=${${REPLY:t2}:-/}
}

_agent_ws_file() {
  print -r -- "${XDG_STATE_HOME:-${HOME%/}/.local/state}/agent-picker/recent"
}

_agent_ws_extras() {
  local file=$(_agent_ws_file) line dir i=0
  local -a f keyed=()
  reply=()
  [[ -r $file ]] || return 0
  for line in ${(f)"$(<$file)"}; do
    f=("${(@ps:\t:)line}")
    [[ $f[1] == $1 ]] || continue
    (( ++i ))
    for dir in $f[3,-1]; do [[ -d $dir ]] || continue 2; done
    keyed+=("${(l:6::0:)i}"$'\t'"${(pj:\t:)f[2,-1]}")
  done
  reply=(${${(O)keyed}#*$'\t'})
}

_agent_ws_record() {
  local file=$(_agent_ws_file) line here=$1 op=$2 n=0 c= at=0
  shift 2
  local -a f kept=()
  [[ -r $file ]] && for line in ${(f)"$(<$file)"}; do
    f=("${(@ps:\t:)line}")
    [[ -d $f[1] ]] || continue
    if [[ $f[1] == $here && ${(pj:\t:)f[3,-1]} == ${(pj:\t:)@} ]]; then
      n=${f[2]%%:*} c=${${(M)f[2]%:*}#:} at=$(( $#kept + 1 ))
      continue
    fi
    kept+=($line)
  done
  case $op in
    bump) (( n++ )); at=$(( $#kept + 1 )) ;;
    tint) c=$(( (${c:-0} + 1) % 7 )); (( at )) || at=$(( $#kept + 1 )) ;;
    del) at=0 ;;
  esac
  (( at )) && kept[at,at-1]=("$here"$'\t'"$n${${c:#0}:+:$c}"${1:+$'\t'}${(pj:\t:)@})
  (( $#kept > 300 )) && shift $(( $#kept - 300 )) kept
  mkdir -p -- ${file:h} && print -rn -- ${(F)kept}${kept:+$'\n'} >| $file.$$ && mv -f -- $file.$$ $file
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

  local here=${PWD:A} tab=recent out dir line hdr op c pos=1 i where
  local -a combos ranked rows lines picked sel checked fields mflags shown
  local -A uses
  _agent_ws_extras $here
  for line in $reply; do
    fields=("${(@ps:\t:)line}")
    for dir in $fields[2,-1]; do uses[$dir]=$(( ${uses[$dir]:-0} + ${fields[1]%%:*} )); done
  done
  shown=(); for dir in ${(k)uses}; do shown+=("${(l:6::0:)uses[$dir]}:$dir"); done
  ranked=(${${(O)shown}#*:})
  _agent_short $here; where=$REPLY

  while true; do
    rows=() checked=() mflags=(--expect=left,right)
    if [[ $tab == recent ]]; then
      _agent_ws_extras $here
      combos=($reply)
      i=${combos[(I)<->(|:<->)]}
      (( i )) && combos=($combos[i] $combos[1,i-1] $combos[i+1,-1]) || combos=(0 $combos)
      for line in $combos; do
        fields=("${(@ps:\t:)line}")
        c=${${(M)fields[1]%:*}#:}
        if (( $#fields > 1 )); then
          shown=(); for dir in $fields[2,-1]; do shown+=(${dir:t}); done
          line="+ ${(j: + :)shown}"
        else
          _agent_short $here; line="◎ $REPLY"
        fi
        [[ -n $c ]] && line=$'\e[3'$c'm'$line$'\e[0m'
        rows+=("$line"$'\t'"${(pj:\t:)fields[2,-1]}")
      done
      mflags=(--ansi --cursor=$pos --expect=left,right,ctrl-d,ctrl-r)
      hdr="[recent]  new →    ^r color · ^d delete"
    else
      for dir in ${(u)sel} ${ranked:|sel}; do
        _agent_short $dir; rows+=("$REPLY"$'\t'"$dir")
        (( ${sel[(Ie)$dir]} )) && checked+=($#rows)
      done
      (( $+commands[zoxide] )) && for dir in ${(f)"$(zoxide query -l | grep -vxFf <(print -rl -- $here $ranked $sel))"}; do
        [[ -e $dir/.git ]] || continue
        _agent_short $dir; rows+=("$REPLY"$'\t'"$dir")
      done
      mflags+=(--multi --checked=${(j:,:)checked})
      hdr="← recent  [new]    space pick · enter launch"
    fi
    out="$(print -rl -- $rows | winnow $mflags --print-query \
        --delimiter=$'\t' --with-nth=1 --height=20 --prompt="$selected ($where) > " --header=$hdr)" || (( $? == 1 )) || { zle reset-prompt; return 0 }

    lines=("${(@f)out}")
    picked=(${(ps:\t:)${(pj:\t:)${lines[3,-1]#*$'\t'}}})
    case $lines[2] in
      left|right)
        [[ $tab == new ]] && sel=($picked) tab=recent || tab=new
        pos=1
        continue ;;
      ctrl-d|ctrl-r)
        (( $#lines > 2 )) || continue
        pos=${rows[(ie)$lines[3]]}
        [[ $lines[2] == ctrl-d ]] && op=del || op=tint
        _agent_ws_record $here $op $picked
        continue ;;
    esac
    dir=${lines[1]/#\~/${HOME%/}}
    if [[ $lines[1] == [/~]* && -d $dir && $dir != *$'\t'* ]]; then
      [[ $tab == new ]] && sel=($picked)
      sel+=(${dir:A}) tab=new
      continue
    fi
    (( $#lines > 2 )) && break
  done

  picked=(${(o)${(u)picked:#$here}})
  _agent_ws_record $here bump $picked
  local -a extras=($picked)

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

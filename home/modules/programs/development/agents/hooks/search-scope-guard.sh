input=$(cat)
tool=$(jq -r '.tool_name // empty' <<<"$input")
cwd=$(jq -r '.cwd // empty' <<<"$input")
h=${HOME%/}

# a walk rooted at /, /home, $HOME or a top-level dotdir like ~/.config
too_broad() {
  local p=${1//[\'\"]/}
  case $p in
  "~"*) p=$h${p#\~} ;;
  "\${HOME}"*) p=$h${p#\$\{HOME\}} ;;
  "\$HOME"*) p=$h${p#\$HOME} ;;
  /*) ;;
  *) p=$cwd/$p ;;
  esac
  p=$(realpath -sm -- "$p")
  [[ $p == / || $p == /home || $p == "$h" || ($p == "$h"/.* && $p != "$h"/.*/* && ! -e $p/.git) ]] && [[ -d $p ]]
}

deny() {
  jq -n --arg p "$1" '{hookSpecificOutput: {
    hookEventName: "PreToolUse",
    permissionDecision: "deny",
    permissionDecisionReason: ("Unscoped recursive search over " + $p + " is refused. Name the exact subdirectory or file you need (e.g. ~/.config/nvim, not ~/.config).")
  }}'
  exit 0
}

if [[ $tool == Grep ]]; then
  p=$(jq -r '.tool_input.path // empty' <<<"$input")
  if too_broad "${p:-.}"; then deny "${p:-$cwd}"; fi
  exit 0
fi

cmd=$(jq -r '.tool_input.command // empty' <<<"$input")
[[ -n $cmd ]] || exit 0

set -f
readarray -t segs < <(tr ';&|()`' '\n' <<<"$cmd")
for seg in "${segs[@]}"; do
  read -ra words <<<"$seg"
  i=0
  while ((i < ${#words[@]})); do
    case ${words[i]} in
    sudo | env | xargs | command | exec | time | nice | -* | *=*) ((i += 1)) ;;
    *) break ;;
    esac
  done
  ((i < ${#words[@]})) || continue
  case ${words[i]##*/} in
  grep | egrep | fgrep | rg | ugrep | ug | ag | ack) ;;
  find) [[ $seg =~ -(exec|execdir|ok|okdir)[[:space:]] ]] || continue ;;
  *) continue ;;
  esac
  for w in "${words[@]:i+1}"; do
    [[ $w == -* ]] && continue
    if too_broad "$w"; then deny "$w"; fi
  done
done

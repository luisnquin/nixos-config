if [[ -n ${HERDR_ENV:-} ]]; then
  pane=${HERDR_ACTIVE_PANE_ID:-$(herdr pane list | jq -r '.result.panes[] | select(.focused) | .pane_id' | head -1)}
  text() { herdr pane send-text "$pane" "$1" >/dev/null; }
  key() { herdr pane send-keys "$pane" "$1" >/dev/null; }
elif [[ -n ${TMUX:-} ]]; then
  pane=$(tmux display -p '#{pane_id}')
  text() { tmux send-keys -t "$pane" -l "$1"; }
  key() { tmux send-keys -t "$pane" "$([[ $1 == esc ]] && echo Escape || echo Enter)"; }
else
  echo "agent-say: run inside herdr or tmux" >&2
  exit 1
fi
[[ -n $pane ]] || exit 1

out=$(winnow --print-query --expect=alt-enter,ctrl-x --prompt="say > " \
  --header="enter: send  alt-enter: type only  ctrl-x: interrupt, then send  no match: sends what you typed" \
  <"$AGENT_SAY_SNIPPETS") || (($? == 1)) || exit 0

mapfile -t lines <<<"$out"
msg=${lines[2]:-${lines[0]}}
[[ -n $msg ]] || exit 0

if [[ ${lines[1]} == ctrl-x ]]; then
  key esc
  # a key right after Esc is read as Alt+key
  sleep 0.3
fi
text "$msg"
[[ ${lines[1]} == alt-enter ]] || key enter

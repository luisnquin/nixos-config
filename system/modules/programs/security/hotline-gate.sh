# i hate this shit

export HERDR_SESSION=hub

# A pty and no command is the phone's SHELL tab: the login shell, as sshd
# would start it without the forced command. Everything else stays gated.
if [ -z "${SSH_ORIGINAL_COMMAND:-}" ] && [ -t 0 ]; then
  exec "${SHELL:-/bin/sh}" -l
fi

read -ra argv <<< "${SSH_ORIGINAL_COMMAND:-agent list}"
if [ "${argv[0]:-}" = herdr ]; then
  argv=("${argv[@]:1}")
fi
if [ "${argv[0]:-}" != agent ]; then
  echo "denied: ${SSH_ORIGINAL_COMMAND:-}" >&2
  exit 2
fi
verb=${argv[1]:-}
pane=${argv[2]:-}

pane_ok() { [[ $1 =~ ^[A-Za-z0-9]+:[A-Za-z0-9]+$ ]]; }
key_ok() { [[ $1 =~ ^(enter|esc|up|down|tab|[A-Za-z0-9])$ ]]; }
projection() {
  jq -c '[.result.agents[]? | {pane_id, agent, agent_status, state_change_seq, terminal_title_stripped, workspace_id, cwd}]'
}

case $verb in
  list)
    exec herdr agent list
    ;;
  watch)
    last=
    while :; do
      if ! snapshot=$(herdr agent list); then
        printf '%s\n' "$snapshot"
        exit 1
      fi
      current=$(printf '%s' "$snapshot" | projection)
      if [ "$current" != "$last" ]; then
        printf '%s\n' "$snapshot"
        last=$current
      fi
      sleep 2
    done
    ;;
  read)
    pane_ok "$pane" || { echo "denied: pane" >&2; exit 2; }
    exec herdr agent read "$pane" --source visible --lines 8 --format text
    ;;
  keys)
    pane_ok "$pane" || { echo "denied: pane" >&2; exit 2; }
    keys=("${argv[@]:3}")
    if [ ${#keys[@]} -lt 1 ]; then
      echo "denied: no keys" >&2
      exit 2
    fi
    for key in "${keys[@]}"; do
      key_ok "$key" || { echo "denied: key $key" >&2; exit 2; }
    done
    exec herdr agent send-keys "$pane" "${keys[@]}"
    ;;
  spawn)
    kind=${argv[2]:-}
    dir="${argv[*]:3}"
    [[ $kind =~ ^(claude|codex)$ ]] || { echo "denied: kind" >&2; exit 2; }
    dir=${dir/#\~/$HOME}
    { [ -n "$dir" ] && [ -d "$dir" ]; } || { echo "denied: dir" >&2; exit 2; }
    text=$(cat)
    [ -n "$text" ] || { echo "denied: empty prompt" >&2; exit 2; }
    label=$(basename "$dir")
    if ! created=$(herdr workspace create --cwd "$dir" --label "$label" --no-focus); then
      printf '%s\n' "$created" >&2
      exit 1
    fi
    pane=$(printf '%s' "$created" | jq -r '.result.root_pane.pane_id')
    [[ $pane =~ ^[A-Za-z0-9]+:[A-Za-z0-9]+$ ]] || { echo "spawn: no pane in $created" >&2; exit 1; }
    case $kind in
      claude) flags=(--dangerously-skip-permissions) ;;
      codex) flags=(--yolo) ;;
    esac
    # A workspace whose agent never came up is closed again: the phone gets
    # the exit code, not a stray empty space in the hub.
    if ! herdr agent start "$label" --kind "$kind" --pane "$pane" -- "${flags[@]}" >/dev/null \
      || ! herdr agent prompt "$pane" "$text" >/dev/null; then
      herdr workspace close "${pane%%:*}" >/dev/null 2>&1 || true
      exit 1
    fi
    printf '%s\n' "$pane"
    ;;
  *)
    echo "denied: agent $verb" >&2
    exit 2
    ;;
esac

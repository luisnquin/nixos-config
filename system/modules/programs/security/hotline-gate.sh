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
  paths)
    # Every git checkout the phone may send to, so a path is picked from a list
    # instead of typed from memory on a phone keyboard. `~` because the home
    # that matters is this host's, and the app stores what it is shown.
    roots=()
    for root in "$HOME/Projects" "$HOME/.dotfiles" "$HOME/src"; do
      if [ -d "$root" ]; then roots+=("$root"); fi
    done
    # `.git` is a directory in a plain clone and a *file* in a submodule or a
    # worktree, so matching on the type drops every submodule a superproject
    # holds. Pruning is what keeps the walk out of the object stores.
    printf '%s\n' "${roots[@]}" | xargs -r -I{} find {} -mindepth 1 -maxdepth 6 -name .git -prune -printf '%h\n' 2>/dev/null \
      | sed "s|^$HOME|~|" | sort -u | head -300 \
      | jq -R -s -c 'split("\n") | map(select(length > 0))'
    ;;
  models)
    # What the phone may pick from, asked of the machine the agents run on:
    # codex enumerates its own bundled slugs, and claude has no equivalent verb,
    # so it gets the three aliases that always resolve to the current model.
    case ${argv[2]:-} in
      codex) codex debug models --bundled | jq -c '[.models[].slug]' ;;
      claude) printf '["opus","sonnet","fable"]\n' ;;
      *) echo "denied: kind" >&2; exit 2 ;;
    esac
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
    model=${argv[3]:-}
    dir="${argv[*]:4}"
    [[ $kind =~ ^(claude|codex)$ ]] || { echo "denied: kind" >&2; exit 2; }
    # `-` is the agent's own default. Anything else reaches the agent's argv, so
    # it is matched whole and may not open with a dash.
    [[ $model == - || $model =~ ^[a-z0-9][a-z0-9._:-]{0,63}$ ]] || { echo "denied: model" >&2; exit 2; }
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
    # An agent name is lowercase, starts with a letter and is unique in the
    # session, none of which a directory basename owes it: `.dotfiles` is
    # refused outright and the second send from a path collides with the first.
    name=${label,,}
    name=${name//[^a-z0-9_-]/-}
    name=${name#"${name%%[a-z]*}"}
    [ -n "$name" ] || name=agent
    name=${name:0:24}-${pane%%:*}
    name=${name,,}
    case $kind in
      # The bypass disclaimer is a startup screen no unattended agent gets past.
      # `--settings` is a settings source of its own, so accepting it holds for
      # this run alone and nothing on disk is flipped.
      claude) flags=(--settings '{"skipDangerousModePermissionPrompt":true}' --dangerously-skip-permissions) ;;
      codex) flags=(--yolo) ;;
    esac
    [ "$model" = - ] || flags+=(--model "$model")
    # A workspace whose agent never came up is closed again: the phone gets
    # the exit code, not a stray empty space in the hub.
    if ! herdr agent start "$name" --kind "$kind" --pane "$pane" -- "${flags[@]}" >/dev/null \
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

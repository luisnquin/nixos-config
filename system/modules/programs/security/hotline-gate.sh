# i hate this shit

export HERDR_SESSION=hub

# An ssh login starts with none of the desktop's environment, and neither
# does an exec channel: the sockets are found under the runtime directory and
# /tmp when nothing names them.
clip_env() {
  runtime="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}"
  wayland="${WAYLAND_DISPLAY:-}"
  if [ -z "$wayland" ]; then
    for s in "$runtime"/wayland-*; do
      case $s in *.lock) continue ;; esac
      [ -S "$s" ] && wayland=${s##*/} && break
    done
  fi
  display="${DISPLAY:-}"
  if [ -z "$display" ]; then
    for s in /tmp/.X11-unix/X*; do
      [ -S "$s" ] && display=":${s##*/X}" && break
    done
  fi
}

# A pty and no command is the phone's SHELL tab: the login shell, as sshd
# would start it without the forced command, plus the display this desktop
# is on. An agent started in it reads the clipboard the phone writes to; with
# no WAYLAND_DISPLAY arboard falls to X11 and dies on a socket it cannot
# reach. One display, not both: Claude Code asks xclip before wl-paste, and
# on a Wayland desktop the X11 side is Xwayland's bridge, which answers for
# the selection only while an X11 window has focus. Everything else stays
# gated.
if [ -z "${SSH_ORIGINAL_COMMAND:-}" ] && [ -t 0 ]; then
  clip_env
  export XDG_RUNTIME_DIR="$runtime"
  if [ -n "$wayland" ]; then
    export WAYLAND_DISPLAY="$wayland"
  elif [ -n "$display" ]; then
    export DISPLAY="$display"
  fi
  exec "${SHELL:-/bin/sh}" -l
fi

read -ra argv <<< "${SSH_ORIGINAL_COMMAND:-agent list}"
if [ "${argv[0]:-}" = herdr ]; then
  argv=("${argv[@]:1}")
fi

# CLIPS: a picture the phone lands under /tmp/hotline and puts on this
# desktop's clipboard, so an agent under the SHELL tab takes it on Ctrl+V.
# The verbs mirror the `sh -c` scripts the app runs on a host with no gate,
# and `probe` prints in the same shape, so the app parses both alike.
clip_dir=/tmp/hotline
clip_name_ok() { [[ $1 =~ ^[A-Za-z0-9_-]+\.png$ ]]; }
if [ "${argv[0]:-}" = clip ]; then
  name=${argv[2]:-}
  case ${argv[1]:-} in
    probe)
      clip_env
      printf 'os=%s\n' "$(uname -s)"
      printf 'runtime=%s\nwayland=%s\ndisplay=%s\nxauthority=%s\n' \
        "$runtime" "$wayland" "$display" "${XAUTHORITY:-}"
      for t in wl-copy wl-paste xclip osascript timeout; do
        command -v "$t" >/dev/null 2>&1 && printf 'has=%s\n' "$t"
      done
      # 124 is the timeout killing a watch that was running, which only a
      # compositor with data-control lets it do; anything else is the refusal.
      if [ -n "$wayland" ] && command -v wl-paste >/dev/null 2>&1; then
        watch=0
        error=$(XDG_RUNTIME_DIR="$runtime" WAYLAND_DISPLAY="$wayland" timeout 1 wl-paste --watch true 2>&1 >/dev/null) || watch=$?
        printf 'watch=%s\nwatcherror=%s\n' "$watch" "$error"
      fi
      ;;
    put)
      clip_name_ok "$name" || { echo "denied: clip name" >&2; exit 2; }
      mkdir -p "$clip_dir"
      head -c 33554432 > "$clip_dir/$name"
      ;;
    copy)
      clip_name_ok "$name" || { echo "denied: clip name" >&2; exit 2; }
      [ -f "$clip_dir/$name" ] || { echo "clip: no $name" >&2; exit 1; }
      clip_env
      # The tool forks to go on serving the selection, so nothing of the
      # channel may stay in its hands: both outputs are dropped here.
      if [ -n "$wayland" ]; then
        XDG_RUNTIME_DIR="$runtime" WAYLAND_DISPLAY="$wayland" wl-copy --type image/png < "$clip_dir/$name" >/dev/null 2>&1
      elif [ -n "$display" ]; then
        DISPLAY="$display" xclip -selection clipboard -t image/png -i "$clip_dir/$name" >/dev/null 2>&1
      else
        echo "clip: no display server" >&2
        exit 1
      fi
      ;;
    *)
      echo "denied: clip ${argv[1]:-}" >&2
      exit 2
      ;;
  esac
  exit 0
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

now=$(date +%s)
host=$(uname -n)

cyan=$'\033[36m'
green=$'\033[32m'
yellow=$'\033[33m'
dim=$'\033[2m'
reset=$'\033[0m'

plural() {
  if [ "$1" -eq 1 ]; then
    printf '%s %s' "$1" "$2"
  else
    printf '%s %ss' "$1" "$2"
  fi
}

rel() {
  local last=$1 d

  if [ "$last" -le 0 ]; then
    printf 'never'
    return
  fi

  d=$((now - last))

  if [ "$d" -lt 60 ]; then
    printf 'now'
  elif [ "$d" -lt 3600 ]; then
    printf '%dm ago' $((d / 60))
  elif [ "$d" -lt 86400 ]; then
    printf '%dh ago' $((d / 3600))
  else
    printf '%dd ago' $((d / 86400))
  fi
}

herdr_session=${1:-hyprland}
herdr_color=$dim
herdr_note="$herdr_session down"
herdr_tree=''

running=$(herdr session list --json 2>/dev/null | jq -r --arg s "$herdr_session" '.sessions[]? | select(.name == $s) | .running' 2>/dev/null || true)

if [ "$running" = 'true' ]; then
  # the cli reports api failures as json on stderr, so both streams are kept
  snapshot=$(herdr --session "$herdr_session" api snapshot 2>&1 || true)
  failure=$(printf '%s' "$snapshot" | jq -r '.error.code // empty' 2>/dev/null || true)

  if [ -n "$failure" ]; then
    herdr_color=$yellow
    herdr_note="$herdr_session up · api $failure"
  elif ! printf '%s' "$snapshot" | jq -e . >/dev/null 2>&1; then
    herdr_color=$yellow
    herdr_note="$herdr_session up · api unreadable"
  else
    body=$(printf '%s' "$snapshot" | jq -c '.result.snapshot // .snapshot // .' 2>/dev/null || true)

    panes=$(printf '%s' "$body" | jq -r '(.panes // []) | length' 2>/dev/null || echo 0)
    statuses=$(printf '%s' "$body" | jq -r '.agents[]?.agent_status' 2>/dev/null || true)

    total_agents=$(printf '%s' "$statuses" | grep -c . || true)
    blocked_agents=$(printf '%s' "$statuses" | grep -c '^blocked$' || true)
    working_agents=$(printf '%s' "$statuses" | grep -c '^working$' || true)

    herdr_note="$herdr_session up · $(plural "$panes" pane)"

    if [ "$total_agents" -gt 0 ]; then
      herdr_note="$herdr_note · $(plural "$total_agents" agent)"
    fi

    if [ "$blocked_agents" -gt 0 ]; then
      herdr_color=$yellow
      herdr_note="$herdr_note · $blocked_agents blocked"
    fi

    if [ "$working_agents" -gt 0 ]; then
      herdr_note="$herdr_note · $working_agents working"
    fi

    workspaces=$(printf '%s' "$body" | jq -r '.workspaces[]? | [(if .focused then ">" else " " end), ((.label // "?")[0:14]), (.tab_count // 0), (.pane_count // 0), (.agent_status // "unknown")] | @tsv' 2>/dev/null || true)

    shown_workspaces=0
    total_workspaces=0

    while IFS=$'\t' read -r focus label tabs workspace_panes status; do
      [ -n "$label" ] || continue

      total_workspaces=$((total_workspaces + 1))

      if [ "$shown_workspaces" -lt 4 ]; then
        shown_workspaces=$((shown_workspaces + 1))

        line=$(printf '      %s %-14s %-7s %-9s %s' "$focus" "$label" "$(plural "${tabs:-0}" tab)" "$(plural "${workspace_panes:-0}" pane)" "$status")

        if [ -z "$herdr_tree" ]; then
          herdr_tree="$line"
        else
          herdr_tree="$herdr_tree
$line"
        fi
      fi
    done <<<"$workspaces"

    if [ "$total_workspaces" -gt "$shown_workspaces" ]; then
      herdr_tree="$herdr_tree
$(printf '        +%d more' $((total_workspaces - shown_workspaces)))"
    fi
  fi
fi

sessions=$(tmux list-sessions -F '#{session_last_attached}|#{session_name}|#{session_windows}|#{session_attached}' 2>/dev/null | sort -rn || true)

note='no sessions'

if [ -n "$sessions" ]; then
  shown=()
  total=0

  while IFS='|' read -r last name windows attached; do
    [ -n "$name" ] || continue

    total=$((total + 1))

    if [ "$total" -le 3 ]; then
      mark=''
      if [ "$attached" -gt 0 ]; then
        mark='*'
      fi

      shown+=("$name$mark (${windows}w, $(rel "$last"))")
    fi
  done <<<"$sessions"

  note=''

  for s in "${shown[@]}"; do
    if [ -z "$note" ]; then
      note="$s"
    else
      note="$note · $s"
    fi
  done

  if [ "$total" -gt 3 ]; then
    note="$note · +$((total - 3)) more"
  fi
fi

failed_system=$(systemctl --failed --no-legend --plain 2>/dev/null | wc -l || true)
failed_user=$(systemctl --user --failed --no-legend --plain 2>/dev/null | wc -l || true)
failed=$((failed_system + failed_user))

authfails=$(journalctl -u sshd.service --since today --no-pager --output=cat -g 'Failed|Invalid user' 2>/dev/null | wc -l || true)

warn=''

if [ "$failed" -gt 0 ]; then
  warn="$failed failed unit"

  if [ "$failed" -ne 1 ]; then
    warn="${warn}s"
  fi
fi

if [ "$authfails" -gt 0 ]; then
  if [ -n "$warn" ]; then
    warn="$warn · "
  fi

  warn="$warn$authfails authfails today"
fi

printf '\n'
printf '  %sincoming ssh%s · %s\n\n' "$dim" "$reset" "$host"
printf '  %s[h]%s herdr %s— %s%s\n' "$cyan" "$reset" "$herdr_color" "$herdr_note" "$reset"

if [ -n "$herdr_tree" ]; then
  printf '%s%s%s\n' "$dim" "$herdr_tree" "$reset"
fi

printf '  %s[t]%s tmux %s— %s%s\n' "$green" "$reset" "$dim" "$note" "$reset"
printf '  %s[z]%s zsh\n' "$yellow" "$reset"

if [ -n "$warn" ]; then
  printf '\n  %s! %s%s\n' "$yellow" "$warn" "$reset"
fi

#!/usr/bin/env bash

set -euo pipefail

state="${1:-}"
title="${2:-}"

case "$state" in
clear) progress=0 ;;
working) progress=3 ;;
error) progress=2 ;;
waiting) progress=4 ;;
*)
	echo "agent-terminal-status: invalid state: $state" >&2
	exit 2
	;;
esac

if ! { exec 3>/dev/tty; } 2>/dev/null; then
	exit 0
fi

emit() {
	local sequence="$1"

	if [ -n "${TMUX:-}" ]; then
		sequence="${sequence//$'\033'/$'\033\033'}"
		printf '\033Ptmux;\033%s\033\134' "$sequence" >&3
	else
		printf '%s' "$sequence" >&3
	fi
}

if [ -n "$title" ]; then
	emit "$(printf '\033]9;3;%s\033\134' "$title")"
else
	emit "$(printf '\033]9;3;\033\134')"
fi

emit "$(printf '\033]9;4;%s\033\134' "$progress")"

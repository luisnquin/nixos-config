#!/usr/bin/env bash
#
# agent-notify - cancelable/delayable desktop + ntfy notifications backed by
# transient systemd --user timers.
#
# The delay lives locally (in a one-shot systemd timer named after --id) so a
# later event can cancel it before it fires. ntfy server-side scheduling cannot
# be canceled, which is why the previous DELETE-based approach never worked.
#
# Subcommands:
#   schedule --id ID --delay DELAY --title T --message M --image IMG [--ntfy-url URL]
#       Arm a timer that runs `fire` after DELAY, replacing any pending timer
#       with the same ID.
#   cancel --id ID
#       Stop a pending timer for ID (no-op if none is armed).
#   fire --title T --message M --image IMG [--ntfy-url URL]
#       Emit the notification now (desktop via notify-send, remote via ntfy).
#       Invoked by the timer; can also be called directly for immediate sends.

set -euo pipefail

action="${1:-}"
if [ -z "$action" ]; then
  echo "agent-notify: missing subcommand" >&2
  exit 2
fi
shift

id=""
delay=""
title=""
message=""
image=""
ntfy_url=""

while [ "$#" -gt 0 ]; do
  case "$1" in
    --id) id="$2"; shift 2 ;;
    --delay) delay="$2"; shift 2 ;;
    --title) title="$2"; shift 2 ;;
    --message) message="$2"; shift 2 ;;
    --image) image="$2"; shift 2 ;;
    --ntfy-url) ntfy_url="$2"; shift 2 ;;
    *) echo "agent-notify: unknown argument: $1" >&2; exit 2 ;;
  esac
done

unit=""
if [ -n "$id" ]; then
  unit="notify-$id"
fi

# Tear down any existing units for this id. --collect already reaps finished
# ones, but a still-pending timer would make systemd-run fail with
# "unit already exists", so we stop+reset defensively. Failures are ignored:
# the unit simply may not exist.
cleanup_unit() {
  systemctl --user stop "$unit.timer" "$unit.service" >/dev/null 2>&1 || true
  systemctl --user reset-failed "$unit.timer" "$unit.service" >/dev/null 2>&1 || true
}

do_fire() {
  notify-send -a "$title" -i "$image" "$title" "$message" || true
  if [ -n "$ntfy_url" ]; then
    curl -fsS -H "Title: $title" -d "$message" "$ntfy_url" >/dev/null 2>&1 || true
  fi
}

case "$action" in
  schedule)
    if [ -z "$unit" ]; then
      echo "agent-notify: schedule requires --id" >&2
      exit 2
    fi
    if [ -z "$delay" ]; then
      echo "agent-notify: schedule requires --delay" >&2
      exit 2
    fi

    cleanup_unit

    # Re-invoke ourselves at fire time. readlink -f resolves the store path we
    # were called by so systemd-run gets an absolute, PATH-independent exec.
    self="$(readlink -f "$0")"

    fire_args=(fire --title "$title" --message "$message" --image "$image")
    if [ -n "$ntfy_url" ]; then
      fire_args+=(--ntfy-url "$ntfy_url")
    fi

    # AccuracySec=1s: systemd timers default to 1min accuracy, which would make
    # a "10s" delay fire up to a minute late.
    # --setenv DBUS_SESSION_BUS_ADDRESS: the user manager may not export it, but
    # notify-send needs the session bus we were invoked with.
    systemd-run \
      --user \
      --quiet \
      --collect \
      --unit="$unit" \
      --on-active="$delay" \
      --timer-property=AccuracySec=1s \
      --setenv=DBUS_SESSION_BUS_ADDRESS="${DBUS_SESSION_BUS_ADDRESS:-}" \
      -- "$self" "${fire_args[@]}"
    ;;

  cancel)
    if [ -z "$unit" ]; then
      echo "agent-notify: cancel requires --id" >&2
      exit 2
    fi
    cleanup_unit
    ;;

  fire)
    do_fire
    ;;

  *)
    echo "agent-notify: unknown subcommand: $action" >&2
    exit 2
    ;;
esac

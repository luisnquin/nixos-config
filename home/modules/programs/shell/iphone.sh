# Grab the iPhone connected to the mac (rose) and put the PNG in the clipboard.
#
# iOS 17+ only exposes the screenshot service over a RemoteXPC tunnel, so rose
# runs `pymobiledevice3 remote tunneld` as a root launchd daemon that keeps a
# tunnel up over whichever transport is available (USB or WiFi).
#
# The client MUST be aimed at that tunnel with `--tunnel <UDID>`. The bare
# `screenshot` command defaults to "the first USB device", so it dies with
# "Device is not connected" whenever the phone is only reachable over WiFi
# (e.g. right after a mac reboot, before USB re-enumerates) even though tunneld
# already holds a working tunnel. Going through tunneld works on any transport
# and needs no daemon restart. The UDID is the first key of tunneld's HTTP list.
#
# pymobiledevice3 can exit 0 even on some failures, so success is decided by
# whether a non-empty PNG actually came back, not by the remote exit code.
copy_ios_screenshot() {
  local tmp remote
  tmp="$(mktemp --suffix=.png)" || return 1

  remote='udid="$(curl -s http://127.0.0.1:49151/ | python3 -c "import sys,json;d=json.load(sys.stdin);print(next(iter(d)))" 2>/dev/null)"; [ -n "$udid" ] || exit 1; d="$(mktemp -d)"; pymobiledevice3 developer dvt screenshot --tunnel "$udid" "$d/shot.png" >/dev/null 2>&1; cat "$d/shot.png" 2>/dev/null; rm -rf "$d"'

  ssh rose "$remote" >"$tmp" 2>/dev/null
  if [ ! -s "$tmp" ]; then
    # tunneld may still be bringing the tunnel up right after a mac reboot.
    sleep 3
    ssh rose "$remote" >"$tmp" 2>/dev/null
  fi

  if [ -s "$tmp" ]; then
    wl-copy --type image/png <"$tmp"
    rm -f "$tmp"
  else
    print -ru2 -- "copy_ios_screenshot: capture failed (no tunnel; is the iPhone connected to rose and unlocked?)"
    rm -f "$tmp"
    return 1
  fi
}

stream_ios_logs() {
  local bundle_id remote
  bundle_id="$1"

  if [ -z "$bundle_id" ]; then
    print -ru2 -- "usage: stream_ios_logs <bundle-id>"
    return 2
  fi

  if [[ "$bundle_id" == *[^A-Za-z0-9._-]* ]]; then
    print -ru2 -- "stream_ios_logs: invalid bundle ID"
    return 2
  fi

  remote='udid="$(curl -s http://127.0.0.1:49151/ | python3 -c "import sys,json;d=json.load(sys.stdin);print(next(iter(d)))" 2>/dev/null)"; [ -n "$udid" ] || { echo "stream_ios_logs: no iPhone tunnel" >&2; exit 1; }; pid="$(pymobiledevice3 developer dvt process-id-for-bundle-id --tunnel "$udid" "$1" 2>/dev/null)"; case "$pid" in (""|*[!0-9]*) echo "stream_ios_logs: app is not running: $1" >&2; exit 1;; esac; exec pymobiledevice3 developer dvt oslog --tunnel "$udid" "$pid"'

  ssh -t rose "sh -c '$remote' sh '$bundle_id'"
}

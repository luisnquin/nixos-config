#!/usr/bin/env zsh

# Grab a screenshot off a connected Android device/emulator into the clipboard.
#
# adb refuses to act when more than one target is attached ("more than one
# device/emulator"), so this picks one: a bare call lists every device in the
# `device` state and, when there is more than one, prompts with fzf; an optional
# argument pre-selects by (partial) serial or model and skips the prompt.
# offline/unauthorized targets are dropped since they cannot serve a capture.
#
# Capture goes through `exec-out` rather than `shell` so the raw PNG bytes skip
# the pty's LF->CRLF translation that corrupts binary output. screencap can warn
# and still exit 0, so success is decided by a valid PNG signature coming back,
# not by adb's exit code.
#
# `exec-out` merges the device's stderr into the same stream as its stdout, so a
# device-side warning lands *inside* the PNG. Suppressing it needs the redirect
# to run on the device (inside the quoted command), not on the local adb.
#
# Foldables expose several internal displays and screencap defaults to "the
# first one found", which is not stable across captures and is usually the panel
# that is currently off. The powered-on display is picked explicitly.
copy_android_screenshot() {
    local want="$1" tmp serial sel r display
    local -a rows

    rows=("${(@f)$(adb devices -l 2>/dev/null | awk '
    NR>1 && $2=="device" {
      serial=$1; model=""; product="";
      for (i=3;i<=NF;i++){ split($i,a,":");
        if(a[1]=="model") model=a[2]; if(a[1]=="product") product=a[2] }
      desc=(model!="")?model:serial;
      if(product!="") desc=desc" ("product")";
      printf "%s\t%s\n", serial, desc
    }')}")

    if [ ${#rows[@]} -eq 0 ]; then
        print -ru2 -- "copy_android_screenshot: no device online (check: adb devices)"
        return 1
    fi

    if [ -n "$want" ]; then
        for r in "${rows[@]}"; do
            case "$r" in *"$want"*)
                serial="${r%%$'\t'*}"
                break
                ;;
            esac
        done
        if [ -z "$serial" ]; then
            print -ru2 -- "copy_android_screenshot: no online device matching '$want'"
            return 1
        fi
    elif [ ${#rows[@]} -eq 1 ]; then
        serial="${rows[1]%%$'\t'*}"
    else
        sel="$(printf '%s\n' "${rows[@]}" | fzf --delimiter='\t' --with-nth=2.. \
            --prompt='device> ' --header='select android target' --height=40% --reverse)" || return 1
        [ -n "$sel" ] || return 1
        serial="${sel%%$'\t'*}"
    fi

    tmp="$(mktemp --suffix=.png)" || return 1

    display="$(adb -s "$serial" shell dumpsys SurfaceFlinger --displays 2>/dev/null |
        tr -d '\r' |
        awk '/^Display [0-9]+$/ { id = $2 } /powerMode=On$/ { print id; exit }')"

    adb -s "$serial" exec-out "screencap -p ${display:+-d $display} 2>/dev/null" >"$tmp" 2>/dev/null

    if [ -s "$tmp" ] && [ "$(od -An -tx1 -N4 "$tmp" | tr -d ' \n')" = "89504e47" ]; then
        wl-copy --type image/png <"$tmp"
        rm -f "$tmp"
        print -ru2 -- "copy_android_screenshot: copied from $serial"
    else
        print -ru2 -- "copy_android_screenshot: capture failed on $serial (is it awake and unlocked?)"
        rm -f "$tmp"
        return 1
    fi
}

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

# `phone` drives the devices on this desk: Android handsets and emulators
# reachable over adb, and an iPhone that is only reachable through the mac
# (rose).
#
# Every command keys off a single device list whose rows are
# <platform>\t<id>\t<label>, so target matching, the fzf prompt and the
# per-platform capture paths stay independent of each other. Emulators get
# their own platform rather than being folded into `android`: they are the
# noisiest part of the list and the one you most often want to exclude.

IOS_HOST=rose
TUNNELD_URL=http://127.0.0.1:49151/

TMP=""

cleanup() {
    [ -z "$TMP" ] || rm -f "$TMP"
}

trap cleanup EXIT

die() {
    printf 'phone: %s\n' "$*" >&2
    exit 1
}

usage() {
    cat <<'EOF'
usage: phone <command> [target] [args]

commands:
  devices                  list every reachable device
  shot    [target]         screenshot to the clipboard
  logs    [target] <app>   stream logs for a package name / bundle id
  mirror  [target]         scrcpy mirror (adb targets only)

options:
  -o, --out FILE           write the capture to FILE instead of the
                           clipboard; "-" writes the raw PNG to stdout
  -h, --help               show this

platforms: android (handset), emu (avd), ios

<target> is any substring of a platform, serial or label -- "emu",
"ios", "faraday", an AVD name. Omit it and phone prompts with fzf
when more than one device matches.
EOF
}

list_android() {
    local serial model platform label

    while read -r serial model; do
        case $serial in
            emulator-*)
                platform=emu
                # `model` is the system image (sdk_gphone64_x86_64) for every
                # AVD alike; the console reports the name the nix config
                # declared, which is the one worth showing.
                label=$(adb -s "$serial" emu avd name 2>/dev/null | tr -d '\r' | awk 'NR == 1' || true)
                [ -n "$label" ] || label=$model
                ;;
            *)
                platform=android
                label=$model
                ;;
        esac

        printf '%s\t%s\t%s (%s)\n' "$platform" "$serial" "$label" "$serial"
    done < <(adb devices -l 2>/dev/null | awk '
        NR > 1 && $2 == "device" {
            model = $1

            for (i = 3; i <= NF; i++) {
                split($i, kv, ":")
                if (kv[1] == "model") model = kv[2]
            }

            print $1, model
        }
    ' || true)
}

list_ios() {
    ssh -o BatchMode=yes -o ConnectTimeout=3 "$IOS_HOST" \
        "curl -s --max-time 3 $TUNNELD_URL" 2>/dev/null |
        jq -r --arg host "$IOS_HOST" 'keys[] | "ios\t\(.)\tiPhone (\($host))"' 2>/dev/null ||
        true
}

list_devices() {
    local droid ios

    droid=$(mktemp)
    ios=$(mktemp)

    # the iPhone probe is an ssh round trip; overlap it with the local adb call
    list_android >"$droid" &
    list_ios >"$ios" &
    wait

    cat "$droid" "$ios"
    rm -f "$droid" "$ios"
}

# select_device [target] [platform] -> one <platform>\t<id>\t<label> row
select_device() {
    local want=${1-} only=${2-} row sel
    local -a rows matched

    mapfile -t rows < <(list_devices)

    if [ -n "$only" ]; then
        matched=()
        for row in "${rows[@]}"; do
            if [[ ${row%%$'\t'*} =~ ^($only)$ ]]; then
                matched+=("$row")
            fi
        done
        rows=("${matched[@]}")
    fi

    [ "${#rows[@]}" -gt 0 ] || die "no phone reachable (check: adb devices, ssh $IOS_HOST)"

    if [ -n "$want" ]; then
        matched=()
        for row in "${rows[@]}"; do
            case ${row,,} in *"${want,,}"*) matched+=("$row") ;; esac
        done
        [ "${#matched[@]}" -gt 0 ] || die "no phone matching '$want'"
        rows=("${matched[@]}")
    fi

    if [ "${#rows[@]}" -eq 1 ]; then
        printf '%s\n' "${rows[0]}"
        return 0
    fi

    # fzf reads the candidates from this pipe but draws on /dev/tty, so the
    # prompt survives being run inside a command substitution or a pipeline.
    sel=$(printf '%s\n' "${rows[@]}" |
        fzf --delimiter=$'\t' --with-nth=3.. --prompt='phone> ' \
            --header='select a device' --height=40% --reverse) ||
        die "cancelled"

    printf '%s\n' "$sel"
}

capture_android() {
    local serial=$1 display

    # Foldables expose several internal panels and screencap defaults to "the
    # first display found", which is not stable across captures and is usually
    # the panel that is currently off.
    display=$(adb -s "$serial" shell dumpsys SurfaceFlinger --displays 2>/dev/null |
        tr -d '\r' |
        awk '/^Display [0-9]+$/ { id = $2 } /powerMode=On$/ { print id; exit }' || true)

    # exec-out keeps the pty's LF->CRLF translation away from the PNG, but it
    # also folds the device's stderr into the same stream: the redirect has to
    # run on the device, inside the quoted command, not on the local adb.
    adb -s "$serial" exec-out "screencap -p ${display:+-d $display} 2>/dev/null" || true
}

capture_ios() {
    local udid=$1 remote

    case $udid in *[!A-Za-z0-9-]*) die "invalid udid: $udid" ;; esac

    # iOS 17+ only serves screenshots over a RemoteXPC tunnel, kept up by the
    # `pymobiledevice3 remote tunneld` daemon on rose. The bare `screenshot`
    # command targets "the first USB device" and dies whenever the phone is only
    # reachable over WiFi, so it has to be aimed at the tunnel explicitly.
    # shellcheck disable=SC2016
    remote='d="$(mktemp -d)"; pymobiledevice3 developer dvt screenshot --tunnel "$1" "$d/shot.png" >/dev/null 2>&1; cat "$d/shot.png" 2>/dev/null; rm -rf "$d"'

    ssh -o BatchMode=yes "$IOS_HOST" "sh -c '$remote' sh '$udid'" 2>/dev/null || true
}

capture() {
    case $1 in
        android | emu) capture_android "$2" ;;
        ios) capture_ios "$2" ;;
    esac
}

# screencap and pymobiledevice3 both exit 0 on some failures, so a capture only
# counts once a PNG signature actually comes back.
emit() {
    local out=$1 magic

    [ -s "$TMP" ] || return 1

    magic=$(od -An -tx1 -N4 "$TMP" | tr -d ' \n')
    [ "$magic" = 89504e47 ] || return 1

    case $out in
        "") wl-copy --type image/png <"$TMP" ;;
        -) cat "$TMP" ;;
        *) cp -- "$TMP" "$out" ;;
    esac
}

logs_android() {
    local serial=$1 app=$2 pid

    pid=$(adb -s "$serial" shell pidof "$app" 2>/dev/null | tr -d '\r' | awk '{print $1}' || true)
    [ -n "$pid" ] || die "app is not running: $app"

    exec adb -s "$serial" logcat --pid="$pid"
}

logs_ios() {
    local udid=$1 app=$2 remote

    case $udid in *[!A-Za-z0-9-]*) die "invalid udid: $udid" ;; esac

    # shellcheck disable=SC2016
    remote='pid="$(pymobiledevice3 developer dvt process-id-for-bundle-id --tunnel "$1" "$2" 2>/dev/null)"; case "$pid" in ""|*[!0-9]*) echo "phone: app is not running: $2" >&2; exit 1;; esac; exec pymobiledevice3 developer dvt oslog --tunnel "$1" "$pid"'

    exec ssh -t "$IOS_HOST" "sh -c '$remote' sh '$udid' '$app'"
}

cmd_devices() {
    local platform id label found=0

    while IFS=$'\t' read -r platform id label; do
        printf '%-9s %-28s %s\n' "$platform" "$id" "$label"
        found=1
    done < <(list_devices)

    [ "$found" -eq 1 ] || die "no phone reachable (check: adb devices, ssh $IOS_HOST)"
}

cmd_shot() {
    local want="" out="" platform id label row

    while [ $# -gt 0 ]; do
        case $1 in
            -o | --out)
                [ $# -ge 2 ] || die "--out needs a path"
                out=$2
                shift 2
                ;;
            -h | --help)
                usage
                return 0
                ;;
            -*) die "unknown option: $1" ;;
            *)
                want=$1
                shift
                ;;
        esac
    done

    row=$(select_device "$want")
    IFS=$'\t' read -r platform id label <<<"$row"

    TMP=$(mktemp --suffix=.png)
    capture "$platform" "$id" >"$TMP"

    if ! emit "$out"; then
        [ "$platform" = ios ] || die "capture failed on $label (is it awake and unlocked?)"

        # tunneld may still be bringing the tunnel up right after a mac reboot
        sleep 3
        capture "$platform" "$id" >"$TMP"
        emit "$out" || die "capture failed on $label (is the tunnel up?)"
    fi

    # stdout carries PNG bytes under `-o -` and nothing else, ever
    [ "$out" = - ] || printf 'phone: captured %s\n' "$label" >&2
}

cmd_logs() {
    local want="" app="" platform id row

    case $# in
        1) app=$1 ;;
        2)
            want=$1
            app=$2
            ;;
        *) die "usage: phone logs [target] <package-or-bundle-id>" ;;
    esac

    case $app in
        *[!A-Za-z0-9._-]*) die "invalid app id: $app" ;;
    esac

    row=$(select_device "$want")
    IFS=$'\t' read -r platform id _ <<<"$row"

    case $platform in
        android | emu) logs_android "$id" "$app" ;;
        ios) logs_ios "$id" "$app" ;;
    esac
}

cmd_mirror() {
    local want=${1-} id row

    row=$(select_device "$want" 'android|emu')
    IFS=$'\t' read -r _ id _ <<<"$row"

    exec scrcpy -s "$id"
}

case ${1-} in
    devices)
        shift
        cmd_devices "$@"
        ;;
    shot)
        shift
        cmd_shot "$@"
        ;;
    logs)
        shift
        cmd_logs "$@"
        ;;
    mirror)
        shift
        cmd_mirror "$@"
        ;;
    "" | -h | --help | help) usage ;;
    *) die "unknown command: $1 (try: phone --help)" ;;
esac

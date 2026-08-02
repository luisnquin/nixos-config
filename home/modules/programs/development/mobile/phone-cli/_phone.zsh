#compdef phone

local -a commands targets

commands=(
    'devices:list every reachable device'
    'shot:screenshot to the clipboard'
    'logs:stream logs for a package name / bundle id'
    'mirror:scrcpy mirror (adb targets only)'
)

if (( CURRENT == 2 )); then
    _describe -t commands 'phone command' commands
    return
fi

if [[ ${words[CURRENT-1]} == (-o|--out) ]]; then
    _files
    return
fi

# adb is local and cheap; the iPhone sits behind an ssh round trip to the mac,
# so it is offered as a literal rather than probed on every tab.
targets=(
    android emu ios
    ${(f)"$(adb devices -l 2>/dev/null | awk 'NR > 1 && $2 == "device" { print $1 }')"}
)

case ${words[2]} in
    shot|logs|mirror)
        (( CURRENT == 3 )) && _describe -t targets 'target' targets
        ;;
esac

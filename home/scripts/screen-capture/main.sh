#!/bin/sh

NOTIFICATION_ICON_PATH="/path/to/icon"

capture_and_notify() {
    maim_args="$1"
    notification_body="$2"

    screenshot_path="/tmp/$(uuidgen).png"

    # normally failed exit code when user cancel capture
    if eval "maim $maim_args" >"$screenshot_path"; then
        notify-send 'Screenshot taken!' "$notification_body" --icon="$NOTIFICATION_ICON_PATH"
        xclip -sel clip -t image/png "$screenshot_path"
    fi

    rm -f "$screenshot_path"
}

main() {
    set -e

    case "$1" in
    --selection)
        capture_and_notify '--select' 'From selection'
        ;;

    --active-window)
        capture_and_notify '--window $(xdotool getactivewindow)' 'From active window'
        ;;

    --screen)
        capture_and_notify '' 'From screen'
        ;;

    -* | *)
        echo "not valid option provided"
        echo ""
        echo "screen-capture [flags]"
        echo ""
        echo "Flags:"
        echo " --selection     save user selection in clipboard"
        echo " --active-window save current window in clipboard"
        echo " --screen        save screen in clipboard"

        exit 1
        ;;

    esac
}

main "$@"

#!/bin/sh

NOTIFICATION_ICON_PATH="/path/to/icon"

main() {
    set -e

    screenshot_path="/tmp/$(uuidgen).png"

    case "$1" in
    --selection)
        if maim --select >"$screenshot_path"; then
            xclip -sel clip -t image/png "$screenshot_path"

            notify-send 'Screenshot taken!' 'From selection' --icon="$NOTIFICATION_ICON_PATH"
        fi

        rm -f "$screenshot_path"
        ;;

    --active-window)
        if maim --window "$(xdotool getactivewindow)" >"$screenshot_path"; then
            xclip -sel clip -t image/png "$screenshot_path"

            notify-send 'Screenshot taken!' 'From active window' --icon="$NOTIFICATION_ICON_PATH"
        fi

        rm -f "$screenshot_path"
        ;;

    --screen)
        if maim >"$screenshot_path"; then
            xclip -sel clip -t image/png "$screenshot_path"

            notify-send 'Screenshot taken!' 'From screen' --icon="$NOTIFICATION_ICON_PATH"
        fi

        rm -f "$screenshot_path"
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

#!/bin/sh

# https://www.reddit.com/r/i3wm/comments/2bhhog/mediakeys_spotify_for_noobs/

PROGRAM_NAME="spotify-dbus"

DESTINY="org.mpris.MediaPlayer2.spotify"

main() {
    COMMAND=""

    case "$1" in
    --next)
        COMMAND="Next"
        ;;
    --prev | --previous)
        COMMAND="Previous"
        ;;
    --toggle)
        COMMAND="PlayPause"
        ;;
    --stop)
        COMMAND="Stop"
        ;;
    *)
        echo "Usage"
        echo "$PROGRAM_NAME [flag]"
        echo "Flags:"
        echo "--next               Send a dbus message to pass to the next song"
        echo "--prev | --previous  Send a dbus message to go to the previous song"
        echo "--toggle             Send a dbus message to play or pause the current song"
        echo "--stop               Send a dbus message to just stop"
        exit 1
        ;;
    esac

    MESSAGE="org.mpris.MediaPlayer2.Player.$COMMAND"
    MESSAGE_PATH="/org/mpris/MediaPlayer2"

    dbus-send --print-reply --dest="$DESTINY" "$MESSAGE_PATH" "$MESSAGE"
}

main "$@"

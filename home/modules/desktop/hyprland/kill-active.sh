#!/bin/sh

if hyprctl activewindow | grep "class.*google-chat-linux"; then
    pid=$(hyprctl activewindow | grep pid | awk '{print $2}')
    kill -9 "$pid"
    return
fi

hyprctl dispatch killactive

# xdotool getactivewindow windowunmap
# hyprctl dispatch closewindow "$(echo "$active_window" | grep pid | tr -d " \t\n\r")"

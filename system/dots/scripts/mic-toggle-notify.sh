#!/bin/sh

# For now it requires manual configuration for KDE

microphone_state=''

if [[ $(output=$(amixer set Capture toggle); echo "$output" | grep -m 1 --fixed-strings --only-matching '[on]') ]]; then
  microphone_state=unmuted;
else
  microphone_state=muted; fi

notify-send "Global microphone state: $microphone_state" "Triggered by keybinding (Ctrl + Alt + M)" --app-name=System --expire-time=2500 --urgency=normal

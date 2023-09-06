#!/bin/sh

ASSETS_PATH="path/to/assets"

PROGRAM_NAME="dunstify-sound"
LEVEL_OFFSET=4

SOURCE_TAG="string:x-dunst-stack-tag:source"
SINK_TAG="string:x-dunst-stack-tag:sink"

BAR_COLOR="bar-color-placeholder"

main() {
    case "$1" in
    --inc)
        amixer set Master $LEVEL_OFFSET%+
        notify_audio_update
        ;;
    --dec)
        amixer set Master $LEVEL_OFFSET%-
        notify_audio_update
        ;;
    --toggle-vol)
        amixer set Master toggle
        notify_audio_update
        ;;
    --toggle-mic)
        pactl set-source-mute @DEFAULT_SOURCE@ toggle
        notify_microphone_update
        ;;
    *)
        echo "Usage:"
        echo "$PROGRAM_NAME [flag]"
        echo "Flags:"
        echo "  --inc			Increase speaker's volume."
        echo "  --dec			Decrease speaker's volume."
        echo "  --toggle-mic	Mute/unmute speaker."
        echo "  --toggle-vol:	Mute/unmute microphone."
        exit 1
        ;;
    esac
}

notify_audio_update() {
    vol_state=$(amixer sget Master | awk -F"[][]" '/Left:/ { print $4 }')
    vol_level=$(amixer sget Master | awk -F"[][]" '/Left:/ { print $2 }' | cut -d '%' -f 1)

    sink_icon_path=""

    if [ "$vol_state" = "off" ] || [ "$vol_level" = 0 ]; then
        sink_icon_path="$ASSETS_PATH/volume-off.512.png"
    else
        if [ "$vol_level" -lt 34 ]; then
            sink_icon_path="$ASSETS_PATH/volume-low.512.png"
        elif [ "$vol_level" -lt 67 ]; then
            sink_icon_path="$ASSETS_PATH/volume-medium.512.png"
        else
            sink_icon_path="$ASSETS_PATH/volume-high.512.png"
        fi
    fi

    dunstify -h "int:value:$vol_level" \
        -h "string:hlcolor:$BAR_COLOR" \
        -u low \
        -h "$SINK_TAG" \
        -i "$sink_icon_path" \
        "Volume:" \
        "$vol_level%"
}

notify_microphone_update() {
    notification_message=""
    source_icon_path=""

    if pactl list sources | grep -qi 'Mute: yes'; then
        source_icon_path="$ASSETS_PATH/mic-off.512.png"
        notification_message="Muted"
    else
        source_icon_path="$ASSETS_PATH/mic-on.512.png"
        notification_message="Unmuted"
    fi

    dunstify -u low \
        -h $SOURCE_TAG \
        -i "$source_icon_path" \
        "Microphone" \
        "$notification_message"
}

main "$@"

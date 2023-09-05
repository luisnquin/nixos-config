#!/bin/sh

ASSETS_PATH="path/to/assets"

PROGRAM_NAME="dunstify-brightness"
LEVEL_OFFSET=4

BRIGHTNESS_TAG="string:x-dunst-stack-tag:brightness"

main() {
    case "$1" in
    --inc)
        brightnessctl set $LEVEL_OFFSET%+
        notify
        ;;

    --dec)
        brightnessctl set $LEVEL_OFFSET%-
        notify
        ;;
    *)
        echo "Usage:"
        echo "$PROGRAM_NAME [flag]"
        echo "Flags:"
        echo "  --inc	Increase brightness by raw value."
        echo "  --dec	Decrease brightness by raw value."
        ;;
    esac
}

notify() {
    level=$(brightnessctl -m | awk -F"," '{print $4}' | cut -d '%' -f 1)
    laptop_icon_path="$ASSETS_PATH/laptop.512.png"

    dunstify -h "int:value:$level" -h string:hlcolor:"#ebdbb2" \
        -h $BRIGHTNESS_TAG -u low -i "$laptop_icon_path" \
        "Brightness:" \
        "$level"
}

main "$@"

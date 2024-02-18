# Change between 60 and 100, additionally you can indicate an specific limit
batlimit() {
    new_limit=$1

    if [[ "$new_limit" != "" ]] && [[ ! "$new_limit" =~ ^([1-9]|[1-9][0-9]|100)$ ]]; then
        printf '\033[31mEnter a number between 1 and 100.\033[0m\n'
        return 1
    fi

    current_limit=$(cat /sys/class/power_supply/BAT1/charge_control_end_threshold)

    if [[ $new_limit == '' ]]; then
        if [[ $current_limit == '61' ]]; then
            new_limit='100'
        else
            new_limit='61'
        fi
    fi

    sudo echo "$new_limit" | sudo tee /sys/class/power_supply/BAT1/charge_control_end_threshold >/dev/null

    if ! sudo -nv 2>/dev/null; then
        return 1
    fi

    printf '\033[38;2;92;94;93mOld: \033[0m%s  \033[38;2;39;219;129mNew: \033[0m%s\n' "$current_limit" "$new_limit"
}

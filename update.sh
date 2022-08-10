#!/usr/bin/sh

reboot=0
turnoff=0

# TODO: receive flags to restart or shutdown at the end
# Flags parsing
while [ "$#" -gt 0 ]; do
    case "$1" in
    -r | --r | -reboot | --reboot) reboot=1 shift 1 ;;
    -t | --t | -turnoff | --turnoff) turnoff=1 shift 1 ;;
    -h | --h | -help | --help)
        printf "sh %s [flags]\n\nFlags:\n-r, --reboot\tReboots the machine after the update\n-t, --turnoff\tPower off the machine after the update\n-h, --help\tHelp for the script\n" "$0"
        shift 1
        exit 0
        ;;

    -*)
        printf "unknown option: %s\nRun 'sh %s --help' for usage\n" "$1" "$0" >&2
        exit 1
        ;;
    esac
done

main() {
    set -e

    printf "Welcome, \033[1;34m%s\033[0m! ❄️❄️❄️\n\n" "$USER"
    check_fs

    sudo nixos-rebuild boot --upgrade --show-trace

    if [ "$reboot" = 1 ]; then
        reboot
        exit 0
    elif [ "$turnoff" = 1 ]; then
        poweroff
        exit 0
    fi

    printf "\n\033[1;34mSuccessfully updated!\033[0m\n\nPress enter to continue"
    read -r
    clear

    end_menu
}

check_fs() {
    devdirectory="$HOME/.dotfiles/"

    config_file="${devdirectory}configuration.nix"
    hardware_file="${devdirectory}hardware-configuration.nix"

    stat "$config_file" >/dev/null
    stat "$hardware_file" >/dev/null
}

wait_for() {
    i=$1
    while [ "$i" -gt 0 ]; do
        printf "%s seconds left\n" "$i"
        i=$((i - 1))
        sleep 1
    done
}

end_menu() {
    printf "Select an option:\n\t\033[0;34m1)\033[0m Turn Off\n\t\033[0;34m2)\033[0m Reboot\n\t\033[0;34m?)\033[0m Do nothing\nOption: "
    read -r answer

    case "$answer" in
    "1")
        printf "\n\033[0;35mTurning off\033[0m ❄️❄️❄️\n"
        wait_for 5
        poweroff
        ;;
    "2")
        printf "\n\033[0;35mRebooting\033[0m ❄️❄️❄️\n"
        wait_for 5
        reboot
        ;;
    *)
        printf "\nBye! ❄️❄️❄️\n"
        exit 0
        ;;
    esac

}

main

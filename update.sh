#!/usr/bin/sh

main() {
    set -e

    printf "Welcome, \033[1;34m%s\033[0m! ❄️❄️❄️\n\n" "$USER"
    check_fs

    sudo nixos-rebuild boot --upgrade --show-trace

    printf "\n\033[1;34mSuccessfully updated!\033[0m\n\nPress enter to continue"
    read -r
    clear
    printf "Do you want to reboot?\033[1;33m(y/N)\033[0m "
    read -r answer

    if [ "$answer" = "y" ] || [ "$answer" = "Y" ]; then
        echo "Rebooting, you probably won't see this ❄️❄️❄️"
        reboot
    else
        echo "Bye! ❄️❄️❄️"
    fi
}

check_fs() {
    devdirectory="$HOME/.dotfiles/"

    config_file="${devdirectory}configuration.nix"
    hardware_file="${devdirectory}hardware-configuration.nix"

    stat "$config_file" >/dev/null
    stat "$hardware_file" >/dev/null
}

main

# TODO: 1 to reboot, 2 to turn off and else to just end the script

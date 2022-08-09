#!/usr/bin/sh

main() {
    printf "Welcome, \033[1;34m%s\033[0m! ❄️❄️❄️\n\n" "$USER"
    
    configuration_devdirectory="$HOME/.dotfiles/"
    configuration_devfile="${configuration_devdirectory}configuration.nix"
    
    stat "$configuration_devfile" > /dev/null
    sudo nixos-rebuild boot --upgrade --show-trace
    
    
    printf "\n\033[1;34mSuccessfully updated!\033[0m\n\nPress enter to continue"; read -r
    clear
    printf "Do you want to reboot?\033[1;33m(y/N)\033[0m "; read -r answer
    
    if [ "$answer" = "y" ] || [ "$answer" = "Y" ];
    then echo "Rebooting, you probably won't see this ❄️❄️❄️"; reboot
    else
        echo "Bye! ❄️❄️❄️"
    fi
}

main

# TODO: 1 to reboot, 2 to turn off and else to just end the script

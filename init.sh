#!/usr/bin/sh

# Improve introduction


configuration_devdirectory="$HOME/.dotfiles/"
configuration_devfile="${configuration_devdirectory}configuration.nix"
configuration_proddirectory="/etc/nixos/"


stat "$configuration_devfile" > /dev/null

sudo cp "$configuration_devfile" "$configuration_proddirectory"

sudo nixos-rebuild boot --upgrade --show-trace


printf "\n\033[1;34mSuccessfully updated!\033[0m\n\nPress enter to continue"
read -r

clear

echo "Do you want to reboot?(y/N)" # Mix this two lines
read -r answer

if [ "$answer" = "y" ] || [ "$answer" = "Y" ];
then reboot
else
    echo "Bye! ❄️❄️❄️"
fi

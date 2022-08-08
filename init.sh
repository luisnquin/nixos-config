#!/usr/bin/sh

configuration_devfile="$HOME/.dotfiles/configuration.nix"
configuration_proddirectory="/etc/nixos/"

stat "$configuration_devfile" > /dev/null

sudo cp "$configuration_devfile" "$configuration_proddirectory"

sudo nixos-rebuild boot --upgrade --show-trace


printf "\nSuccessfully updated!\nPress enter to continue"
read -r

clear

echo "Do you want to reboot?(y/N)" # Mix this two lines
read -r answer

if [ "$answer" = "y" ] || [ "$answer" = "Y" ];
then echo ""
else
    echo "Bye!"
fi
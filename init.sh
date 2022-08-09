#!/usr/bin/sh

main() {
    exists=$(stat ~/.dotfiles/ 2>/dev/null)

    if [ "$exists" != "" ]; then
        printf "Apparently you already have a dotfiles directory, \033[1;31mdelete it\033[0m if you want to continue\n"
        exit 1
    fi

    git clone https://github.com/luisnquin/nixos-config.git ~/.dotfiles/
    nixos-generate-config --show-hardware-config >~/.dotfiles/hardware-configuration.nix
    sudo ln -s ~/.dotfiles/* /etc/nixos/
    sudo nixos-rebuild switch
}

main

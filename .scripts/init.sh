#!/usr/bin/sh

main() {
    set -e

    check_fs
    setup
}

setup() {
    git clone https://github.com/luisnquin/nixos-config.git ~/.dotfiles/
    nixos-generate-config --show-hardware-config >~/.dotfiles/hardware-configuration.nix
    sudo ln -sf ~/.dotfiles/* /etc/nixos/
    sudo nixos-rebuild switch
}

check_fs() {
    exists=$(stat ~/.dotfiles/ 2>/dev/null)

    if [ "$exists" != "" ]; then
        printf "Apparently you already have a dotfiles directory %s/.dotfiles/, \033[1;31mdelete it\033[0m if you want to continue\n" "$HOME"

        return 1
    fi
}

main

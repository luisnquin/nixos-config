#!/usr/bin/sh

# Variables definitions
dotfiles_dir="$HOME/.dotfiles/"
config_file="${dotfiles_dir}configuration.nix"
hardware_file="${dotfiles_dir}hardware-configuration.nix"

reboot=0
turnoff=0
subcommand=0

template=$(
    printf "\033[0;35mnyx\033[0m [command] [flags]\n\nAvailable commands:\n  update\tUpdates the machine\n  inspect\tVerifies if the configuration.nix file has been changed and not saved to a git repository\n  style\t\tApplies alejandra style to all .nix files\n  ls\t\tList elements in dotfiles directory\n  clean\t\tCleans with the old generations\n  voir\t\tChecks if a new NixOS update is available\n\nGlobal flags:\n-r, --reboot\tReboots the machine before the end of the program\n-t, --turnoff\tPower off the machine before the end of the program\n-h, --help\tHelp for the \033[0;35mnyx\033[0m\n"
)

main() {
    set -e
    check_fs

    case "$subcommand" in
    1)
        # Update
        greet
        check_syntax_errors

        ensure_symlinks
        sudo nixos-rebuild boot --upgrade
        printf "\n\033[1;34mSuccessfully updated! ❄️❄️❄️\033[0m\n"
        ;;
    2)
        # Clean up
        greet

        nb_of_removed_gens=$(($(nix-env --list-generations | wc -l) - 1))
        version=$(nixos-version)

        before=$(df --output=size,used,pcent --human-readable / | tail -n +2)
        sudo nix-collect-garbage --delete-old
        after=$(df --output=size,used,pcent --human-readable / | tail -n +2)

        printf "\nNix version: %s" "$version"
        printf "\nRoot directory:\n"

        if [ "$before" = "$after" ]; then
            printf "\033[0;34m%s <- before\033[0m\n\033[0;34m%s <- after\033[0m" "$before" "$after"
        else
            printf "\033[0;31m%s <- before\033[0m\n\033[0;32m%s <- after\033[0m" "$before" "$after"
        fi

        printf "\nGenerations:\n"
        nix-env --list-generations | head

        if [ "$nb_of_removed_gens" != 0 ]; then
            printf "\n%s generations has been removed\n" "$nb_of_removed_gens"
        fi
        ;;
    3)
        # TODO: Voir
        ;;
    4)
        # Inspect
        stat "$dotfiles_dir/.git/" >/dev/null

        nix_files=$(find "$dotfiles_dir" -type f -name "*.nix")

        # First we need to collect all and then show results

        (
            cd "$dotfiles_dir"

            for file in $nix_files; do
                changes=$(git diff --compact-summary "$file")
                if [ "$changes" != "" ]; then
                    printf "\n%s\n\n" "$changes"
                else
                    printf "no changes | %s\n" "${file##*/}"
                fi
            done
        )
        ;;
    5)
        # Style
        (
            cd "$dotfiles_dir"
            alejandra ./* | head
        )
        ;;
    6) # ls
        ls --color=tty "$dotfiles_dir"
        ;;

    esac

    if [ "$reboot" = 1 ]; then
        reboot
        exit 0
    elif [ "$turnoff" = 1 ]; then
        poweroff
        exit 0
    fi
}

check_syntax_errors() {
    (
        cd "$dotfiles_dir"
        alejandra --check --quiet ./*
    )
}

greet() {
    printf "Welcome, \033[1;34m%s\033[0m! ❄️❄️❄️\n\n" "$USER"
}

check_fs() {
    # Nix files
    stat "$config_file" >/dev/null
    stat "$hardware_file" >/dev/null
}

ensure_symlinks() {
    sudo ln -sf ~/.dotfiles/* /etc/nixos/ # 2>/dev/null
}

# Flags parsing
while [ "$#" -gt 0 ]; do
    case "$1" in
    update) subcommand=1 shift 1 ;;
    clean) subcommand=2 shift 1 ;;
    voir) subcommand=3 shift 1 ;;
    inspect) subcommand=4 shift 1 ;;
    style) subcommand=5 shift 1 ;;
    ls) subcommand=6 shift 1 ;;
    -h | --h | -help | --help)
        echo "$template"
        shift 1
        exit 0
        ;;
    -* | *)
        printf "unknown option: %s\nRun 'sh %s --help' for usage\n" "$1" "$0" >&2
        exit 1
        ;;
    esac

    case "$2" in
    -r | --r | -reboot | --reboot) reboot=1 shift 2 ;;
    -t | --t | -turnoff | --turnoff) turnoff=1 shift 2 ;;
    esac
done

if [ "$subcommand" = 0 ]; then
    echo "$template"
    exit 1
fi

main

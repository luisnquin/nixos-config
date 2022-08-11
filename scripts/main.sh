#!/usr/bin/sh

dotfiles_dir="$HOME/.dotfiles/"
config_file="${dotfiles_dir}configuration.nix"
hardware_file="${dotfiles_dir}hardware-configuration.nix"

check_fs() {
    # Nix files
    stat "$config_file" >/dev/null
    stat "$hardware_file" >/dev/null
}

# Variables definitions
reboot=0
turnoff=0
subcommand=0

template=$(
    printf "sh %s [command] [flags]\n\nAvailable commands:\n  update\tUpdates the machine\n  inspect\tVerifies if the configuration.nix file has been changed and not saved to a git repository\n  clean\t\tCleans with the old generations\n  voir\t\tChecks if a new NixOS update is available\n\nGlobal flags:\n-r, --reboot\tReboots the machine after the update\n-t, --turnoff\tPower off the machine after the update\n-h, --help\tHelp for the script\n" "$0"
)

# Flags parsing
while [ "$#" -gt 0 ]; do
    case "$1" in
    update) subcommand=1 shift 1 ;;
    clean) subcommand=2 shift 1 ;;
    voir) subcommand=3 shift 1 ;;
    inspect) subcommand=4 shift 1 ;;
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

main() {
    set -e
    check_fs

    case "$subcommand" in
    1)
        # Update
        printf "Welcome, \033[1;34m%s\033[0m! ❄️❄️❄️\n\n" "$USER"
        sudo nixos-rebuild boot --upgrade --show-trace
        printf "\n\033[1;34mSuccessfully updated! ❄️❄️❄️\033[0m\n"
        ;;
    2) # Clean up
        printf "Welcome, \033[1;34m%s\033[0m! ❄️❄️❄️\n\n" "$USER"
        sudo nix-collect-garbage --delete-old
        printf "\n¿❄️ ?\n"
        ;;
    3) # TODO: Voir
        ;;
    4) # Inspect
        stat "$dotfiles_dir/.git/" >/dev/null

        nix_files=$(find "$dotfiles_dir" -type f -name "*.nix")

        for file in $nix_files; do
            changes=$(git diff --compact-summary "$file")
            if [ "$changes" != "" ]; then
                printf "\n%s\n\n" "$changes"
            else
                printf "no changes | %s\n" "${file##*/}"
            fi
        done
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

main

# I can't write clean scripts

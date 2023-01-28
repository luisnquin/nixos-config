#!/usr/bin/sh

# Variables definitions
dotfiles_dir="$HOME/.dotfiles/"
config_file="${dotfiles_dir}configuration.nix"
hardware_file="${dotfiles_dir}hardware-configuration.nix"

subcommand=0

template=$(
    printf "\033[38;2;142;41;242mnyx\033[0m [command] [flags]\n\n\e[4mAvailable commands:\e[0m\n  \033[38;2;136;192;208mupdate \033[0m\tUpdates the machine\n  \033[38;2;219;245;76minspect\033[0m\tVerifies if the configuration.nix file has been changed and not saved to a git repository\n  \033[38;2;232;232;232mstyle\033[0m 💅\tApplies alejandra style to all .nix files\n  \033[38;2;143;188;187mls\033[0m\t\tList elements in dotfiles directory\n  \033[38;2;191;97;106mclean\033[0m\t\tCleans with the old generations\n\n\e[4mGlobal flags:\e[0m\n-h, --help\tPrint help information\n"
)

main() {
    set -e
    check_fs

    case "$subcommand" in
    1)
        # Update
        greet

        if [ "$(id -u)" -ne 0 ]; then
            sudo echo -n ""
        fi

        printf "\n\033[0;95mChecking syntax errors [1/3]\033[0m\n"
        check_syntax_and_format || {
            printf "\n\033[0;91msyntax errors detected or files are not correctly formatted!\033[0m\n"

            exit 1
        }

        printf "\n\033[0;95mEnsuring symlinks [2/3]\033[0m\n"
        ensure_symlinks

        printf "\n\033[0;92mBuilding and updating [3/3]\033[0m\n"
        sudo nixos-rebuild boot --upgrade

        printf "\n\033[1;34mSuccessfully updated! ❄️❄️❄️\033[0m\n"
        ;;
    2)
        # Clean up
        greet

        nb_of_removed_gens=$(($(nix-env --list-generations | wc -l) - 1))
        version=$(nixos-version)

        before=$(df --output=size,used,pcent --human-readable / | tail -n +2)
        sudo nix-collect-garbage --delete-old # TODO: steps prompt
        nix-store --delete
        sudo rm -rf /tmp/*
        rm -rf ~/.npm/_npx
        docker system prune
        docker system prune --volumes

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
        exa --icons --sort=type "$dotfiles_dir"
        ;;

    esac
}

check_syntax_and_format() {
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
    sudo rm -rf /etc/nixos/
    sudo mkdir -p /etc/nixos/
    sudo ln -sf ~/.dotfiles/* /etc/nixos/
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
done

if [ "$subcommand" = 0 ]; then
    echo "$template"

    exit 1
fi

main

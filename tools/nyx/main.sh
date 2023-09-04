#!/usr/bin/sh

DOTFILES_DIR_PATH="$HOME/.dotfiles/"
NIX_LOGO_PATH="/path/to/nix-logo.png"
PROGRAM_NAME="nyx"

main() {
    set -e

    case "$1" in
    update)
        case "$2" in
        system | home | all | "")
            update_computer "$2"
            ;;
        -h | --h | -help | --help) ;;
        *)
            help_and_exit_1 "$2"
            ;;
        esac
        ;;
    clean)
        clean_computer
        shift 1
        ;;
    inspect)
        inspect_files
        shift 1
        ;;
    style)
        format_nix_files
        shift 1
        ;;
    -h | --h | -help | --help)
        program_help
        shift 1
        exit 0
        ;;
    -* | *)
        help_and_exit_1 "$1"
        ;;
    esac
}

update_computer() {
    greet
    require_sudo

    printf "\n\033[0;95mChecking syntax errors\033[0m\n"
    check_nix_files_format || {
        printf "\n\033[0;91mSyntax errors detected or files are not correctly formatted!\033[0m\n"
        exit 1
    }

    printf "\n\033[0;95mEnsuring symlinks\033[0m\n"
    ensure_symlinks

    printf "\n\033[0;92mStarting update process...\033[0m\n\n"

    body_message=""

    case "$1" in
    system)
        update_system
        body_message="Your system workspace have been updated."
        ;;
    home)
        update_home
        body_message="Your home workspace have been updated."
        ;;
    all | "")
        update_system
        echo
        update_home
        body_message="Your system and home workspace have been updated."
        ;;
    esac

    notify-send "NixOS update" "$body_message" --icon="$NIX_LOGO_PATH" --app-name="nyx"

    printf "\n\033[1;34mSuccessfully updated! ❄️❄️❄️\033[0m\n"
}

clean_computer() {
    greet

    nb_of_removed_gens=$(($(nix-env --list-generations | wc -l) - 1))
    version=$(nixos-version)

    before=$(df --output=size,used,pcent --human-readable / | tail -n +2)
    sudo nix-collect-garbage --delete-old
    nix-store --delete
    sudo rm -rf /tmp/*
    rm -rf ~/.npm/_npx
    docker system prune --volumes -f

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
}

inspect_files() {
    stat "$DOTFILES_DIR_PATH/.git/" >/dev/null

    nix_files=$(find "$DOTFILES_DIR_PATH" -type f -name "*.nix")

    (
        cd "$DOTFILES_DIR_PATH"

        for file in $nix_files; do
            changes=$(git diff --compact-summary "$file")
            if [ "$changes" != "" ]; then
                printf "\n%s\n\n" "$changes"
            else
                printf "no changes | %s\n" "${file##*/}"
            fi
        done
    )
}

update_system() {
    printf "\033[38;2;240;89;104mUpdating system...\033[0m\n"

    (
        cd "$DOTFILES_DIR_PATH"
        log_command_to_execute "sudo nixos-rebuild" "switch --upgrade --flake ."
        sudo nixos-rebuild switch --upgrade --flake .
    )
}

update_home() {
    printf "\033[38;2;240;89;104mUpdating home...\033[0m\n"

    (
        cd "$DOTFILES_DIR_PATH"
        log_command_to_execute "home-manager" "switch --flake ."
        home-manager switch --flake .
    )
}

format_nix_files() {
    (
        cd "$DOTFILES_DIR_PATH"
        alejandra --quiet ./*.nix
    )
}

check_nix_files_format() {
    (
        cd "$DOTFILES_DIR_PATH"
        alejandra --check --quiet ./*.nix
    )
}

require_sudo() {
    if [ "$(id -u)" -ne 0 ]; then sudo echo -n ""; fi
}

log_command_to_execute() {
    printf "\n\e[38;2;112;112;112m(%s)\033[0;32m %s\033[0m %s\n" "$(basename "$DOTFILES_DIR_PATH")" "$1" "$2"
}

greet() {
    printf "Welcome, \033[1;34m%s\033[0m! ❄️❄️❄️\n" "$USER"
}

ensure_symlinks() {
    sudo rm -rf /etc/nixos/
    sudo mkdir -p /etc/nixos/
    sudo ln -sf ~/.dotfiles/* /etc/nixos/
}

program_help() {
    WHITE_PINK="\e[38;2;232;232;232m"
    SKY_BLUE="\e[38;2;136;192;208m"
    MAGENTA="\e[38;2;154;32;201m"
    ORANGE="\e[38;2;242;161;56m"
    DIRTY="\e[38;2;133;133;133m"
    YELLOW="\e[38;2;219;245;76m"
    RED="\e[38;2;191;97;106m"

    UNDERLINE="\e[4m"
    SPECIAL_END="\e[0m"

    echo "$DIRTY$PROGRAM_NAME$SPECIAL_END [command] [flags]"
    echo
    echo "${UNDERLINE}Available commands:$SPECIAL_END"
    echo "  ${SKY_BLUE}update $SPECIAL_END   Updates your computer using your ${MAGENTA}system${SPECIAL_END} and/or ${ORANGE}home${SPECIAL_END} configuration"
    echo "  ${YELLOW}inspect$SPECIAL_END    Verifies if the configuration.nix file has been changed and not saved to a git repository"
    echo "  ${WHITE_PINK}style$SPECIAL_END 💅   Applies alejandra style to all .nix files"
    echo "  ${RED}clean$SPECIAL_END      Cleans with the old generations"
    echo
    echo "${UNDERLINE}Global flags:${SPECIAL_END}"
    echo " -h, --help    Print help information"
}

help_and_exit_1() {
    printf "unknown option: %s\nRun '%s --help' for usage\n" "$1" "$PROGRAM_NAME" >&2
    exit 1
}

main "$@"

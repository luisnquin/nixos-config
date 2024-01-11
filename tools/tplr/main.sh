#!/bin/sh

if [ -z "$TPLR_FILE_PATH" ]; then
    TPLR_FILE_PATH="${XDG_CONFIG_HOME:-${HOME}.config}/tplr/tree.json"
fi

YELLOW="\033[38;2;245;228;49m"
RED="\033[38;2;245;54;70m"
PINK="\033[38;2;235;77;125m"
END="\033[0m"

log_fatal() {
    MSG="$1"
    shift
    echo -e "${RED}$MSG${*}${END}"
    exit 1
}

get_keys_from_jq_path() {
    cat <"$TPLR_FILE_PATH" | jq -r "$1 | keys[]" | tr '\n' ' '
}

is_jq_object_or_array() {
    answer=$(echo "$1" | jq 'if type == "array" or type == "object" then "yes" else "no" end')
    if [ "$answer" = "yes" ]; then
        return 1
    else
        return 0
    fi
}

is_jq_array() {
    answer=$(echo "$1" | jq 'if type == "array" then "yes" else "no" end')
    if [ "$answer" = "yes" ]; then
        return 1
    else
        return 0
    fi
}

get_jq_path() {
    jq_path=""

    for arg in "$@"; do
        jq_path="$jq_path.$arg"
    done

    echo "$jq_path"
}

error_ambiguous_path() {
    current_path="$1"

    log_fatal "You need to specify a path to your template, options: " "$(get_keys_from_jq_path "$current_path")"
}

# https://stackoverflow.com/questions/226703/how-do-i-prompt-for-yes-no-cancel-input-in-a-linux-shell-script
yes_or_no() {
    printf '%s [Y/n] ' "$1"

    old_stty_cfg=$(stty -g)
    stty raw -echo

    answer=$(while ! head -c 1 | grep -i '[ny]'; do true; done)
    stty "$old_stty_cfg"

    echo "$answer"

    if [ "$answer" = "${answer#[Yy]}" ]; then
        return 1
    else
        return 0
    fi
}

contains() {
    string="$1"
    substring="$2"
    if [ "${string#*"$substring"}" != "$string" ]; then
        return 0 # $substring is in $string
    else
        return 1 # $substring is not in $string
    fi
}

main() {
    set -e

    if [ -z "$1" ]; then
        error_ambiguous_path "."
    fi

    jq_path=$(get_jq_path "$@")
    result=$(cat <"$TPLR_FILE_PATH" | jq -r "$jq_path")

    if ! test -e "$result" && is_jq_object_or_array "$result"; then
        error_ambiguous_path "$jq_path"
    else
        src_dir="$result"
        dst_dir=""

        if yes_or_no "Do you want to apply the template in the current directory?"; then
            dst_dir="$(pwd)"

            if [ "$dst_dir" = "$HOME" ]; then
                log_fatal "The target is your \$HOME directory, we refuse to do this ^^"

            elif test -d "$dst_dir/.git"; then
                if ! yes_or_no "It looks like the current directory contains a local git repository, are you sure?"; then
                    echo "OP cancelled."
                    exit 0
                fi
            fi
        else
            default_name=$(echo "$jq_path" | sed -e 's/^.//' | sed 's/\./-/')

            printf "Give me the project name [%s]: " "$default_name"
            read -r answer

            answer=$(echo "$answer" | awk '{$1=$1;print}')
            if contains "$answer" ".."; then
                log_fatal "It looks like you're trying to get back to a parent folder, sadly that's not possible now"
            elif [ "$answer" = "" ]; then
                project_name="$default_name"
            else
                project_name="$answer"
            fi

            dst_dir="$(pwd)/$project_name"
        fi

        if test -d "$dst_dir"; then
            old_content_path=$(mktemp -d --suffix="-tplr-$(basename "$dst_dir")-$(date +"%H_%M")")

            echo "Copying old content to '$old_content_path'..."

            for x in "$dst_dir/*" "$$dst_dir/.[!.]*" "$$dst_dir/..?*"; do
                if [ -e "$x" ]; then mv -- "$x" "$old_content_path/"; fi
            done

            rm -rf "$dst_dir"
        elif test -f "$dst_dir"; then
            log_fatal "The \"destiny directory\" is a file, no idea about how to proceed here"
        fi

        # TODO: add a report here

        mkdir -p "$dst_dir"
        echo "Applying template..."
        cp -rfT "$src_dir" "$dst_dir"

        if test -f "$dst_dir/go.mod"; then
            PROJECT_NAME=$(basename "$dst_dir")

            cat "$dst_dir/go.mod" | sed -i "s/go.dev/github.com\/$USER\/$PROJECT_NAME/g" "$dst_dir/go.mod" 
        fi
    fi
}

main "$@"

export GID UID

# This keybindings allows for fast navigation from left to right and back.
bindkey '^[[1;5D' backward-word
bindkey '^[[1;5C' forward-word

# Caseless when searching for files/directories
zstyle ':completion:*' matcher-list '' 'm:{a-zA-Z}={A-Za-z}' 'r:|=*' 'l:|=* r:|=*'

# Used by codex zsh plugin
zle -N create_completion # Widget creation
bindkey '^X' create_completion

# When the current working directory changes, run a method that checks for a .env file, then sources it, thanks johnhamelink
autoload -U add-zsh-hook
load_local_env() {
    # check file exists, is regular file and is readable:
    if [ -f .env ] && [ -r .env ]; then
        source .env
    fi
}

load_local_env && add-zsh-hook chpwd load_local_env

pill() {
    environment_file="/home/$USER/.cache/environment-info.json"

    connection_info=$(cat "$environment_file" | jq -r '"\(if .connection.under_a_vpn then " under a vpn" else " no vpn" end), \(.connection.ip_address)"')
    weather_info=$(cat "$environment_file" | jq -r '"🌤 \(.weather.feels_like)° \(.weather.text)"')
    disk_stats=($(df / -h --output=size,used,pcent | sed -n '2p' | awk '{print $1,$2,$3}'))
    current_generation=$(nix-env --list-generations | grep current | awk '{print $1}')
    nixos_version=$(nixos-version | grep -o -E '^[0-9]+\.[0-9]+')
    nix_version=$(nix --version | grep -oP '\d+\.\d+')
    nb_of_failed_units=$(systemctl --failed | grep -c "failed")
    day=$(LC_TIME=en_US.UTF-8 date +%A)
    kernel_info=$(uname -smr)

    printf "\033[38;2;206;232;74m󰘚 $kernel_info\033[0m | \033[38;2;126;51;255m NixOS v$nixos_version / Nix v$nix_version\033[0m %-59s \033[38;2;255;162;13m$connection_info\033[0m\n"
    printf "$day, ${weather_info} %-96s"
    echo "${disk_stats[3]} - ${disk_stats[1]}/${disk_stats[2]}"

    if [[ $nb_of_failed_units != "0" ]]; then printf "%-118s\033[38;2;224;45;60m$nb_of_failed_units failed units\033[0m\n"; else echo; fi

    while read channel; do
        frags=($(echo "$channel" | tr ' ' '\n'))
        echo "\033[38;2;114;243;247m<-${frags[1]}\033[0m \033[38;2;220;130;255m${frags[2]}\033[0m"
    done <<<$(nix-channel --list)
}

if ! test -e /tmp/.info_message_displayed && [ "$TERM_PROGRAM" != "vscode" ]; then
    pill

    echo "" >/tmp/.info_message_displayed
fi

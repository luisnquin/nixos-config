export GID UID

# This keybindings allows for fast navigation from left to right and back.
bindkey '^[[1;5D' backward-word
bindkey '^[[1;5C' forward-word

# Caseless when searching for files/directories
zstyle ':completion:*' matcher-list '' 'm:{a-zA-Z}={A-Za-z}' 'r:|=*' 'l:|=* r:|=*'

# When the current working directory changes, run a method that checks for a .env file, then sources it, thanks johnhamelink
autoload -U add-zsh-hook
load_local_env() {
    # check file exists, is regular file and is readable:
    if [ -f .env ] && [ -r .env ]; then
        source .env
    fi
}

load_local_env
add-zsh-hook chpwd load_local_env

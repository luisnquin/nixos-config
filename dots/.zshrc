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

source <(nao completion zsh); compdef _nao nao

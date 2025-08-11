export GID UID

# This keybindings allows for fast navigation from left to right and back.
bindkey '^[[1;5D' backward-word
bindkey '^[[1;5C' forward-word

# Case-less when searching for files/directories
zstyle ':completion:*' matcher-list '' 'm:{a-zA-Z}={A-Za-z}' 'r:|=*' 'l:|=* r:|=*'

# https://stackoverflow.com/a/11873793
setopt interactivecomments

# On deleting with <Alt> + <Backspace> stops the behavior until finding
# a non-alphanumeric character
# Ref: https://unix.stackexchange.com/questions/313806/zsh-make-altbackspace-stop-at-non-alphanumeric-characters
delete_until_not_alphanumerics() {
    local WORDCHARS='~!#$%^&*(){}[]<>?+;'
    zle backward-delete-word
}

zle -N delete_until_not_alphanumerics

bindkey '\e^?' delete_until_not_alphanumerics

if test -f /home/"$USER"/Work/.bashrc; then source /home/"$USER"/Work/.bashrc; fi

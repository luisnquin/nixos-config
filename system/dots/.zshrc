export GID UID

# This keybindings allows for fast navigation from left to right and back.
bindkey '^[[1;5D' backward-word
bindkey '^[[1;5C' forward-word

# Caseless when searching for files/directories
zstyle ':completion:*' matcher-list '' 'm:{a-zA-Z}={A-Za-z}' 'r:|=*' 'l:|=* r:|=*'

# On deleting with <Alt> + <Backspace> stops the behavior until finding
# a non-alphanumeric character
# Ref: https://unix.stackexchange.com/questions/313806/zsh-make-altbackspace-stop-at-non-alphanumeric-characters
delete_until_not_alphanumerics() {
    local WORDCHARS='~!#$%^&*(){}[]<>?+;'
    zle backward-delete-word
}

zle -N delete_until_not_alphanumerics

bindkey '\e^?' delete_until_not_alphanumerics

stats() {
    fc -l 1 | awk 'BEGIN {FS="[ \t]+|//|"} {print $3}' | sort | uniq -c | sort -nr | head -15
}

# Dedicated completions for pushing tags
gtp() {
    if [[ $1 == "" ]]; then echo "fatal: a tag is required" && return 1; fi

    git push origin $1
}

_gtp() {
    if ! test -d '.git'; then return 1; fi

    local tags=($(git tag --list))
    _values 'tags' ${tags[@]}
}

compdef _gtp gtp

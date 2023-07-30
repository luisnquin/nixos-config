export GID UID

# This keybindings allows for fast navigation from left to right and back.
bindkey '^[[1;5D' backward-word
bindkey '^[[1;5C' forward-word

# Case-less when searching for files/directories
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

find_up() {
    path="$(pwd)"

    while ! [ -e "$path"/"$1" ] && [ -n "$path" ]; do
        path="${path%/*}"
    done

    [ -e "$path"/"$1" ] && echo "$path"/"$1"
}

trans_src_target() {
    trans $1:$2 "$3" | tee -a "/home/$USER/Documents/translations/$1-$2.txt"
}

trans_en_es() {
    trans_src_target "en" "es" "$@"
}

trans_es_en() {
    trans_src_target "es" "en" "$@"
}

trans_en_de() {
    trans_src_target "en" "de" "$@"
}

trans_de_en() {
    trans_src_target "de" "en" "$@"
}

[[ ! -r /home/"$USER"/.opam/opam-init/init.zsh ]] || source /home/"$USER"/.opam/opam-init/init.zsh >/dev/null 2>/dev/null

if test -f /home/"$USER"/Work/init.bash; then
    source /home/"$USER"/Work/init.bash
fi

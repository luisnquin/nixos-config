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

pem() {
    if ! test -e .env; then
        printf "\033[38;2;201;71;71m.env file not found\033[0m\n"
        return 1
    fi

    export $(grep -v '^#' .env | xargs)
    printf "\033[38;2;159;240;72mTaken\! \033[0m\n"
}

find-up() {
    path="$(pwd)"

    while ! [ -e "$path"/"$1" ] && [ -n "$path" ]; do
        path="${path%/*}"
    done

    [ -e "$path"/"$1" ] && echo "$path"/"$1"
}

ports() {
    watch -tn 1 "sudo lsof -i -Pn | grep LISTEN | awk '{print \$1, \$3, \$9}' | column -t -s ' ' | sort | uniq"
}

[[ ! -r /home/"$USER"/.opam/opam-init/init.zsh ]] || source /home/"$USER"/.opam/opam-init/init.zsh >/dev/null 2>/dev/null

trans_ee() {
    trans en:es "$@" | tee -a ~/Documents/translations.txt
}

if test -f /home/"$USER"/Work/init.bash; then
    source /home/"$USER"/Work/init.bash
fi

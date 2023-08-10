_nyx() {
    if [[ "$words[2]" == "update" ]]; then
        compadd home system all
    else
        compadd update ls inspect style clean
    fi
}

compdef _nyx nyx

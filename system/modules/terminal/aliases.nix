{config, ...}: {
  environment.shellAliases = {
    setup = "tmux rename-window \"setup 🦭\" \\; split-window -h \\; split-window -v \\; resize-pane -D 3 \\; selectp -t 0 \\; split-window -v \\; resize-pane -D 3 \\; selectp -t 0 \\; send-keys -t 1 \"btop\" ENTER \\; send-keys -t 3 \"k9s  --readonly\" ENTER; clear";

    runds = "(ds; rm -rf compose/nginx/env.json && make compose-up && make build-fast && make run)";
    v3 = "cd ~/go/src/gitlab.wiserskills.net/wiserskills/v3/";
    ds = "v3 && cd dataserver/";

    # Instant tp to some directories
    dot = "cd ~/.dotfiles/";
    down = "cd ~/Downloads/";
    etc = "cd ~/.etc/";
    gopl = "cd ~/Workspace/playground/go/";
    rustpl = "cd ~/Workspace/playground/rust/";
    pl = "cd ~/Workspace/playground/";
    pr = "cd ~/Workspace/projects/";
    tests = "cd ~/Tests/";
    tmp = "cd /tmp/";

    # Overwriten program calls
    rm = "rm --interactive";
    du = "du --human-readable";
    xclip = "xclip -selection c";
    ls = "exa --icons";
    ll = "exa -l";
    la = "exa -a";
    cat = "bat -p";

    ".." = "cd ..";
    "..." = "cd ../..";
    "~" = "cd /home/$USER/";

    utc-date = "date --rfc-3339=seconds | sed 's/ /T/'";
    lsd = "echo 'lsd? lol'";

    open = "xdg-open";
    rc = "rclone";
    tt = "ranger";
    poff = "poweroff";
    neofetch = "freshfetch";
    whoseport = "netstat -tulpln 2> /dev/null | grep :";
    nyancat = "nyancat --no-counter";
    ale = "alejandra --quiet";
    dud = "du --human-readable --summarize";
    man = "tldr";
    transg = "transgression-tui";
    py = "python3";
    share = "ngrok http";
    top = "btop";
    unrar = "unar";
    mt = "mocktail";

    # https://stackoverflow.com/questions/19331497/set-environment-variables-from-file-of-key-value-pairs
    poisonenv = ''if ! test -e .env; then printf "\033[38;2;212;42;62mNothing can be used to poison the environment\033[0m\n"; return 1; fi; export $(xargs <.env); printf "\033[38;2;156;34;227mPoisoned environment \033[0m\n"'';
  };
}

{
  environment.shellAliases = {
    # Instant tp to some directories
    dot = "cd ~/.dotfiles/";
    down = "cd ~/Downloads/";
    etc = "cd ~/.etc/";
    pr = "cd ~/Projects/";

    tmp = "cd /tmp/";

    # Overwriten program calls
    rm = "rm --interactive";
    du = "du --human-readable";
    ls = "exa --icons";
    sls = "exa --icons -Ta -I=.git";
    ll = "exa -l";
    la = "exa -a";
    cat = "bat -p";

    ".." = "cd ..";
    "..." = "cd ../..";
    "...." = "cd ../../..";
    "....." = "cd ../../../..";
    "~" = "cd /home/$USER/";

    utc-date = "date --rfc-3339=seconds | sed 's/ /T/'";
    lsd = "echo 'lsd? lol'";

    open = "xdg-open";
    rc = "rclone";
    tt = "ranger";
    poff = "poweroff";
    neofetch = "macchina";
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
  };
}

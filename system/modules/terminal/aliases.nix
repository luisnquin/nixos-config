{config, ...}: {
  environment.shellAliases = {
    # Instant tp to some directories
    dot = "cd ~/.dotfiles/";
    down = "cd ~/Downloads/";
    etc = "cd ~/.etc/";
    gopl = "cd ~/Workspace/playground/go/";
    rustpl = "cd ~/Workspace/playground/rust/";
    pl = "cd ~/Workspace/playground/";
    pr = "cd ~/Workspace/projects/";
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
    "...." = "cd ../../..";
    "....." = "cd ../../../..";
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
  };
}

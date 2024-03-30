{
  home.shellAliases = rec {
    g = "git";
    ga = "git add";
    # Add file fragments
    gap = "${ga} --patch";
    ginit = "git init && touch .gitignore";
    gaa = "${ga} --all";

    gb = "git branch";

    gc = "git commit -v";
    gca = "${gc} --amend";
    gcm = "${gc} --message";

    gd = "git diff";
    gds = "${gd} --staged";

    gl = "git log --oneline";
    gls = "${gl} | head -n 10";
    gl1 = "git log --graph --abbrev-commit --decorate --format=format:'%C(bold blue)%h%C(reset) - %C(bold green)(%ar)%C(reset) %C(white)%s%C(reset) %C(dim white)- %an%C(reset)%C(bold yellow)%d%C(reset)' --all";
    gl2 = "git log --graph --abbrev-commit --decorate --format=format:'%C(bold blue)%h%C(reset) - %C(bold cyan)%aD%C(reset) %C(bold green)(%ar)%C(reset)%C(bold yellow)%d%C(reset)%n'' %C(white)%s%C(reset) %C(dim white)- %an%C(reset)' --all";
    glhd = ''A=$(git log -1 --format=%s) && echo "$A" | xargs'';

    gck = "git checkout";
    ggc = "git gc --aggressive";

    gf = "git fetch";
    gfa = "${gf} --all";

    # Git garbage collector over all subdirectories, skipping non-directories and non-git repositories
    gmgc = "find . -maxdepth 1 -type d | xargs -I {} bash -c 'if git -C {} rev-parse --git-dir > /dev/null 2>&1; then git -C {} gc --aggressive; fi'";
    # Pull current branch
    ggpull = "git pull origin --ff-only $(git branch --show-current)";
    # Show current branch in all subdirectories, skipping non-directories and non-git repositories
    gmb = ''find . -maxdepth 1 -type d | xargs -I {} bash -c 'if git -C {} rev-parse --git-dir > /dev/null 2>&1; then printf " ~ \033[0;94m{}\033[0m:"; git -C {} branch --show-current; fi' '';
    # Pull the current branch of all subdirectories, skipping non-directories and non-git repositories
    gmpull = "find . -maxdepth 1 -type d | xargs -I {} bash -c 'if git -C {} rev-parse --git-dir > /dev/null 2>&1; then git -C {} pull; fi'";
    # Push the current branch of all subdirectories, skipping non-directories and non-git repositories
    gmpush = "find . -maxdepth 1 -type d | xargs -I {} bash -c 'if git -C {} rev-parse --git-dir > /dev/null 2>&1; then git -C {} push; fi'";
    # Push current branch
    ggpush = "git push origin $(git branch --show-current)";

    # Git reset but it doesn't output anything
    gr = "git reset -q";
    grv = "git revert";

    # Lists the authors of a directory/file.
    gauthors = "git shortlog -n -s -- ";
    gss = "git status -s";
    grb = "git rebase";

    gs = "git stash";
    gsiu = "${gs} --include-untracked";
    gsp = "${gs} pop";

    gt = "git tag";
    gtd = "${gt} --delete";
    # Lists last 5 tags
    gts = "${gt} --sort=v:refname | tac | head -n 5";
    gcl = "git clone";

    lg = "lazygit";
  };
}

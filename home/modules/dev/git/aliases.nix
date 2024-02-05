{
  home.shellAliases = {
    g = "git";
    ga = "git add";
    # Add file fragments
    gap = "git add --patch";
    ginit = "git init && touch .gitignore";
    gaa = "git add --all";

    gb = "git branch";

    gc = "git commit -v";
    gca = "git commit --amend";
    gcm = "git commit --message";

    gd = "git diff";
    gds = "git diff --staged";

    gl = "git log --oneline";
    gls = "git log --oneline | head -n 10";
    gl1 = "git log --graph --abbrev-commit --decorate --format=format:'%C(bold blue)%h%C(reset) - %C(bold green)(%ar)%C(reset) %C(white)%s%C(reset) %C(dim white)- %an%C(reset)%C(bold yellow)%d%C(reset)' --all";
    gl2 = "git log --graph --abbrev-commit --decorate --format=format:'%C(bold blue)%h%C(reset) - %C(bold cyan)%aD%C(reset) %C(bold green)(%ar)%C(reset)%C(bold yellow)%d%C(reset)%n'' %C(white)%s%C(reset) %C(dim white)- %an%C(reset)' --all";
    glhd = ''A=$(git log -1 --format=%s) && echo "$A" | xargs'';

    gck = "git checkout";
    ggc = "git gc --aggressive";

    gf = "git fetch";
    gfa = "git fetch --all";

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
    gsiu = "git stash --include-untracked";
    gsp = "git stash pop";
    gt = "git tag";
    gtd = "git tag --delete";
    # Lists last 5 tags
    gts = "g tag --sort=v:refname | tac | head -n 5";
    gcl = "git clone";
    lg = "lazygit";
  };
}

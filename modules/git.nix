{
  config,
  pkgs,
  ...
}: let
  owner = import "/etc/nixos/owner.nix";
in {
  programs.git = {
    enable = true;

    lfs.enable = true;
    config = {
      user = {
        name = owner.git.name;
        email = owner.git.deprecatedEmail;
        ${owner.git.username} = owner.git.username;
      };

      init = {
        defaultBranch = "main";
      };

      core = {
        editor = "nano -w";
      };

      color = {
        ui = "auto";
      };

      url = {
        "ssh://git@github.com/" = {
          insteadOf = "https://github.com/";
        };

        "ssh://git@gitlab.com/" = {
          insteadOf = "https://gitlab.com/";
        };
      };

      branch = {
        sort = "object";
      };

      pull = {
        rebase = true;
      };

      fetch = {
        prune = true;
      };

      alias = {
        undo = "reset --soft HEAD^";
        amend = "commit --amend";
      };
    };
  };

  environment = {
    systemPackages = with pkgs; [
      act
      gitlab-runner
      git
      lazygit
      gitui # Better than lazygit?
      pre-commit
      git-ignore
      git-chglog # I'm using conventional commits so
      onefetch
    ];

    shellAliases = {
      g = "git";
      ga = "git add";
      gaa = "git add --all";
      # Git branch to git branch, ha
      gb = "git branch";
      # Git commit and verbose 🤨
      gc = "git commit -v";
      gd = "git diff";
      gds = "git diff --staged";
      gl = "git log --oneline";
      gls = "git log --oneline | head -n 10";
      gl1 = "git log --graph --abbrev-commit --decorate --format=format:'%C(bold blue)%h%C(reset) - %C(bold green)(%ar)%C(reset) %C(white)%s%C(reset) %C(dim white)- %an%C(reset)%C(bold yellow)%d%C(reset)' --all";
      gl2 = "git log --graph --abbrev-commit --decorate --format=format:'%C(bold blue)%h%C(reset) - %C(bold cyan)%aD%C(reset) %C(bold green)(%ar)%C(reset)%C(bold yellow)%d%C(reset)%n'' %C(white)%s%C(reset) %C(dim white)- %an%C(reset)' --all";
      # Normal checkout
      gck = "git checkout";
      # Checkout on all git subdirectories
      gmck = ''checkout_target="$1"; find . -maxdepth 1 -type d | xargs -I {} bash -c "if git -C {} rev-parse --git-dir > /dev/null 2>&1; then git -C {} checkout $checkout_target; fi"; unset checkout_target'';
      # Git garbage collector
      ggc = "git gc --aggressive";
      # Git garbage collector over all subdirectories, skipping non-directories and non-git repositories
      gmgc = "find . -maxdepth 1 -type d | xargs -I {} bash -c 'if git -C {} rev-parse --git-dir > /dev/null 2>&1; then git -C {} gc --aggressive; fi'";
      # Pull current branch
      ggpull = "git pull origin $(git branch --show-current)";
      # Pull the specified branch
      gpull = "git pull origin";
      # Show current branch in all subdirectories, skipping non-directories and non-git repositories
      gmb = ''find . -maxdepth 1 -type d | xargs -I {} bash -c 'if git -C {} rev-parse --git-dir > /dev/null 2>&1; then printf " ~ \033[0;94m{}\033[0m:"; git -C {} branch --show-current; fi' '';
      # Pull the current branch of all subdirectories, skipping non-directories and non-git repositories
      gmpull = "find . -maxdepth 1 -type d | xargs -I {} bash -c 'if git -C {} rev-parse --git-dir > /dev/null 2>&1; then git -C {} pull; fi'";
      # Push the current branch of all subdirectories, skipping non-directories and non-git repositories
      gmpush = "find . -maxdepth 1 -type d | xargs -I {} bash -c 'if git -C {} rev-parse --git-dir > /dev/null 2>&1; then git -C {} push; fi'";
      # Push current branch
      ggpush = "git push origin $(git branch --show-current)";
      # Push the specified branch
      gpush = "git push origin";
      # Git reset but it doesn't output anything
      gr = "git reset -q";
      grb = "git rebase";
      gs = "git stash";
      gsp = "git stash pop";
      gss = "git status -s";
      gt = "git tag";

      lg = "lazygit";
    };
  };
}

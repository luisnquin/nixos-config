{user, ...}: {
  programs.git = {
    enable = true;

    userName = user.fullName;
    userEmail = user.gitEmail;
    ignores = [
      "**/.cache/"
      "**/.idea/"
      "**/.~lock*"
      "**/.direnv/"
      "**/node_modules"
      "**/result"
      "**/result-*"
    ];

    extraConfig = {
      core = {
        editor = "nano -w";
        whitespace = "trailing-space,space-before-tab";
      };

      url = {
        "ssh://git@github.com/" = {
          insteadOf = "https://github.com/";
        };

        "ssh://git@gitlab.com/" = {
          insteadOf = "https://gitlab.com/";
        };
      };

      init.defaultBranch = "main";
      apply.whitespace = "fix";
      branch.sort = "object";
      color.ui = "auto";

      rebase.autoStash = true;
      pull.rebase = true;
      fetch.prune = true;
    };

    aliases = {
      undo = "reset --soft HEAD^";
      amend = "commit --amend";
    };
  };
}

{
  pkgs,
  user,
  ...
}: {
  programs.git = {
    enable = true;

    signing = {
      signByDefault = true;
      key = null; # letting GnuPG to decide what signing key to use depending on commit's author
      signer = "${pkgs.gnupg}/bin/gpg2";
    };

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

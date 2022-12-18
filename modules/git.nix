{config, ...}: let
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
}

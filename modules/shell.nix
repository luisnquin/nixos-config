{config, ...}: {
  programs = {
    zsh = {
      enable = true;

      autosuggestions = {
        enable = true;
        async = true;

        highlightStyle = "fg=#9eadab";
        strategy = [
          "history"
        ];
      };

      syntaxHighlighting = {
        enable = true;
        # styles = {};
      };

      enableBashCompletion = true;
      enableCompletion = true;
    };

    starship = {
      enable = true;

      # Ref: https://starship.rs/config
      settings = {
        custom = {
          gitremote = {
            description = "Display symbol for remote git server";
            shell = ["bash" "--noprofile" "--norc"];
            format = "at $output ";
            command = ''GIT_REMOTE_SYMBOL=$(command git ls-remote --get-url 2> /dev/null | awk '{if ($0 ~ /github/) print ""; else if ($0 ~ /gitlab/) print ""; else if ($0 ~ /bitbucket/) print ""; else if ($0 ~ /git/) print ""; else print ""}'); echo "$GIT_REMOTE_SYMBOL "'';
            when = "git rev-parse --is-inside-work-tree 2> /dev/null";
          };
        };

        format = ''
          $directory''${custom.gitremote}$git_branch$git_commit$c$golang$nodejs$python$rust$nix_shell''${env_var.CLIENT}
          $character
        '';
        scan_timeout = 30;
        command_timeout = 400;
        add_newline = true;

        cmd_duration = {
          min_time = 200;
          show_milliseconds = false;
          format = "it took [$duration]($style) ";
        };

        directory = {
          truncation_length = 2;
          truncate_to_repo = false;
          read_only = "";
          read_only_style = "#454343";
          style = "#8d3beb";
        };

        env_var = {
          disabled = false;

          CLIENT = {
            variable = "CLIENT";
            symbol = "💳 ";
            format = "at [$symbol($env_value)]($style)";
          };
        };

        git_branch = {
          symbol = " ";
          style = "#ebb63b";
        };

        golang = {
          symbol = "ﳑ ";
          detect_extensions = ["go"];
          detect_files = ["go.mod" "go.sum" "go.work" ".go-version"];
          version_format = "v\${major}.\${minor}";
          format = "via [$symbol($version)]($style)";
          style = "#5ddade";
        };

        nix_shell = {
          symbol = "❄️ ";
          style = "#c07bed";
          impure_msg = "impure";
          pure_msg = "pure";
        };
      };
    };
  };
}

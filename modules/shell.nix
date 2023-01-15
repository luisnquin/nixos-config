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
        scan_timeout = 30;
        command_timeout = 400;

        cmd_duration = {
          min_time = 200;
          show_milliseconds = false;
          format = "took [$duration]($style) ";
          # style = "";
        };

        directory = {
          truncation_length = 2;
          truncate_to_repo = false;
          read_only = "🔒";
          read_only_style = "#454343";
        };

        golang = {
          symbol = "ﳑ ";
          detect_extensions = ["go"];
          detect_files = ["go.mod" "go.sum" "go.work" ".go-version"];
          version_format = "v\${major}.\${minor}";
          format = "via [$symbol($version )]($style)";
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

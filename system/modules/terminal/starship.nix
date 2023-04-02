{
  config,
  pkgs,
  ...
}: {
  programs.starship = {
    enable = true;

    settings = {
      custom = {
        go_version_used = {
          description = "Displays 🎉 if the go version used in the current project is the same as the local available";
          shell = ["bash" "--noprofile" "--norc"];
          symbol = "🎉";
          format = " $symbol";
          when = ''test -e go.mod && [ "$(awk '/go/ {print $2; exit}' go.mod)" = "$(go version | awk '{sub(/^go/,"",$3); print $3}' | cut -d '.' -f 1,2)" ]'';
        };

        git_remote = {
          description = "Display symbol for remote git server";
          shell = ["bash" "--noprofile" "--norc"];
          format = "hosted in [$output]($style) ";
          command = ''GIT_REMOTE_SYMBOL=$(command git ls-remote --get-url 2> /dev/null | awk '{if ($0 ~ /github/) print ""; else if ($0 ~ /gitlab/) print ""; else if ($0 ~ /bitbucket/) print ""; else if ($0 ~ /git/) print ""; else print ""}'); echo "$GIT_REMOTE_SYMBOL "'';
          when = "git rev-parse --is-inside-work-tree 2> /dev/null";
          style = "#ededed";
        };

        dotfiles_workspace = {
          description = "Displays the current NixOS version";
          shell = ["bash" "--noprofile" "--norc"];
          format = "via [$symbol($output)]($style)";
          command = ''NIXOS_VERSION=$(nixos-version | grep -o -E '^[0-9]+\.[0-9]+'); NIX_VERSION=$(nix --version | grep -oP '\d+\.\d+'); echo "v$NIXOS_VERSION/$NIX_VERSION"'';
          when = "pwd | grep -q '.dotfiles'";
          style = "#5dd5fc";
          symbol = " ";
        };

        current_client = {
          description = "Diplays the current client in case there's the environment variable";
          shell = ["bash" "--noprofile" "--norc"];
          format = " [$symbol($output)]($style)";
          command = ''echo "($(grep -oP 'CLIENT=\K.*' .env | tr '[:lower:]' '[:upper:]') env)"'';
          when = ''test -e .env && grep -o 'CLIENT' .env'';
          style = "#c319f7";
          symbol = " ";
        };
      };

      format = ''
        $directory''${custom.git_remote}$git_branch$git_state$git_metrics$c$golang''${custom.go_version_used}$nodejs$python$rust$nix_shell''${custom.dotfiles_workspace}''${custom.current_client}
        $character
      '';
      scan_timeout = 30;
      command_timeout = 400;
      add_newline = true;

      character = {
        success_symbol = "[褐](bold green)";
        error_symbol = "[](bold red)";
      };

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

      # Is not working, lol
      env_var = {
        disabled = false;
      };

      git_branch = {
        symbol = " ";
        style = "#ebb63b";
      };

      git_metrics = {
        disabled = false;
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
        symbol = " ";
        style = "#c07bed";
        impure_msg = "impure";
        pure_msg = "pure";
      };
    };
  };
}

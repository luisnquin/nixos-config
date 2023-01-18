{
  config,
  pkgs,
  ...
}: let
  owner = import "/etc/nixos/owner.nix";
in {
  environment.shells = [pkgs.zsh];

  home-manager.users."${owner.username}" = {
    programs.zsh = {
      enable = true;
      plugins = with pkgs; [
        {
          name = "zsh-nix-shell";
          file = "nix-shell.plugin.zsh";
          src = fetchFromGitHub {
            owner = "chisui";
            repo = "zsh-nix-shell";
            rev = "v0.5.0";
            sha256 = "0za4aiwwrlawnia4f29msk822rj9bgcygw6a8a6iikiwzjjz0g91";
          };
        }
      ];
    };
  };

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

      promptInit = ''
        if [ "$TMUX" = "" ] && [ "$TERM_PROGRAM" != "vscode" ]; then
            exec ${pkgs.tmux}/bin/tmux
        fi
      '';

      interactiveShellInit = builtins.readFile ../dots/.zshrc;
    };

    starship = {
      enable = true;

      # Ref: https://starship.rs/config
      settings = {
        custom = {
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

          # Nor this
          current_client = {
            description = "Diplays the current client in case there's the environment variable";
            shell = ["bash" "--noprofile" "--norc"];
            format = "in [$symbol($output)]($style) env";
            when = ''[ -n "''${CLIENT+x}" ]'';
            style = "#c319f7";
            symbol = " ";
          };
        };

        format = ''
          $directory''${custom.git_remote}$git_branch$git_commit$c$golang$nodejs$python$rust$nix_shell''${env_var.CLIENT}''${custom.dotfiles_workspace}''${custom.current_client}
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
  };
}

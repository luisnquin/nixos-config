{
  config,
  pkgs,
  ...
}: let
  owner = import "/etc/nixos/owner.nix";
in {
  console = {
    font = "Lat2-Terminus16";
    keyMap = "es";
  };

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
        {
          name = "zsh-you-should-use";
          file = "you-should-use.plugin.zsh";
          src = fetchFromGitHub {
            owner = "MichaelAquilina";
            repo = "zsh-you-should-use";
            rev = "5b316f4af3ac90e044f386003aacdaa0ad606488";
            sha256 = "192jb680f1sc5xpgzgccncsb98xa414aysprl52a0bsmd1slnyxs";
          };
        }
        {
          name = "Extract";
          file = "extract.sh";
          src = fetchFromGitHub {
            owner = "xvoland";
            repo = "Extract";
            rev = "439e92c5b355b40c36d8a445636d0e761ec08217";
            sha256 = "1yaphcdnpxcdrlwidw47waix8kmv2lb5a9ccmf8dynlwvhyvh1wi";
          };
        }
        {
          name = "zinsults";
          file = "zinsults.plugin.zsh";
          src = fetchFromGitHub {
            owner = "ahmubashshir";
            repo = "zinsults";
            rev = "2963cde1d19e3af4279442a4f67e4c0224341c42";
            sha256 = "1mlb0zqaj48iwr3h1an02ls780i2ks2fkdsb4103aj7xr8ls239b";
          };
        }
      ];
    };

    programs.alacritty = {
      enable = true;
      package = pkgs.alacritty;
    };

    xdg.configFile."alacritty.yml".text = builtins.readFile ../dots/home/alacritty.yml;
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

  environment = {
    # Google search, zsh history search, highlighter for conventional
    # branches and tmux startup in non-vscode editors
    #
    # Shell script code called during interactive shell initialisation.
    # This code is assumed to be shell-independent, which means you
    # should stick to pure sh without sh word split.
    interactiveShellInit = builtins.readFile ../dots/.shrc;

    # sessionVariables = {};

    variables = {
      # The other related config only apply to the build
      NIXPKGS_ALLOW_UNFREE = "1";
      PATH = "$PATH:$GORROT:$GOPATH/bin:/home/$USER/.local/bin";
      GOPATH = "/home/$USER/go";
      EDITOR = "nano";
    };

    shellAliases = {
      # My own
      nyx = "sh ~/.dotfiles/.scripts/main.sh";
      gest = "go clean -testcache && go test -v";
      setup = "tmux rename-window \"setup 🦭\" \\; split-window -h \\; split-window -v \\; resize-pane -D 3 \\; selectp -t 0 \\; split-window -v \\; resize-pane -D 3 \\; selectp -t 0 \\; send-keys -t 2 \"spt\" ENTER \\; send-keys -t 1 \"gotop --nvidia --color=vice\" ENTER \\; send-keys -t 3 \"k9s  --readonly\" ENTER; clear";

      # It's not like you're not needed
      runds = "(ds; rm -rf compose/nginx/env.json && make compose-up && make build && make run)";
      v3 = "cd ~/go/src/gitlab.wiserskills.net/wiserskills/v3/";
      ds = "v3 && cd dataserver/";

      # Instant tp to some directories
      dot = "cd ~/.dotfiles/";
      down = "cd ~/Downloads/";
      etc = "cd ~/.etc/";
      gopl = "cd ~/Workspace/playground/go/";
      pl = "cd ~/Workspace/playground/";
      pr = "cd ~/Workspace/projects/";
      tests = "cd ~/Tests/";
      tmp = "cd /tmp/";
      workspace = "cd ~/Workspace/";

      # Overwriten program calls
      cp = "cp --interactive";
      rm = "rm --interactive";
      mv = "mv --interactive";
      du = "du --human-readable";
      xclip = "xclip -selection c";
      ls = "exa --icons";
      ll = "exa -l";
      la = "exa -a";
      cat = "bat -p";

      # Those who are lazy to write definitely go here
      "~" = "cd /home/$USER/";
      open = "xdg-open";
      rc = "rclone";
      cls = "clear";
      poff = "poweroff";
      actl = "act --list";
      neofetch = "freshfetch";
      whoseport = "netstat -tulpln 2> /dev/null | grep :";
      nyancat = "nyancat --no-counter";
      ale = "alejandra --quiet";
      dud = "du --human-readable --summarize";
      man = "tldr";
      transg = "transgression-tui";
      ns = "nix-shell";
      # Abstraction
      listen = "ngrok http";
      py = "python3";
      share = "ngrok http";
      top = "gotop --nvidia --color=vice";
      unrar = "unar";
    };

    shells = [pkgs.zsh];
  };
}

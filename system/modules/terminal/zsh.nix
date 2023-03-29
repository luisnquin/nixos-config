{
  config,
  pkgs,
  ...
}: let
  owner = import ../../../owner.nix;
in {
  environment.systemPackages = [
    pkgs.zsh-completions
  ];

  programs.zsh = {
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

    interactiveShellInit = builtins.readFile ../../dots/.zshrc;
  };
}

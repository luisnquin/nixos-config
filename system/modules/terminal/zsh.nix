{
  pkgs,
  host,
  ...
}: {
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
    };

    enableBashCompletion = true;
    enableCompletion = true;

    promptInit = ''
      if [ "$TMUX" = "" ] && [ "$TERM_PROGRAM" != "vscode" ]; then
          exec ${pkgs.tmux}/bin/tmux
      fi
    '';

    interactiveShellInit = builtins.readFile (builtins.path {
      name = "${host.name}-system-zshrc-script";
      path = ./dots/.zshrc;
    });
  };
}

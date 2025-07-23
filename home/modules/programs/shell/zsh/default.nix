{
  imports = [
    ./plugins.nix
  ];

  programs.zsh = {
    enable = true;
    enableCompletion = true;
    completionInit = builtins.readFile ./completionInit.zsh + builtins.readFile ../../../../../tools/nyx/completions.zsh;

    history = {
      expireDuplicatesFirst = true;
      ignoreSpace = true;
      save = 10000 * 2;
    };

    # (relative to $HOME)
    dotDir = ".zsh";

    # historySubstringSearch = {
    #   enable = true;
    #   searchUpKey = "^[[A";
    #   searchDownKey = "^[[B";
    # };
  };
}

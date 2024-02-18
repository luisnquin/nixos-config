{
  programs.zsh.initExtra = builtins.readFile (builtins.path {
    name = "git-shrc";
    path = ./dots/shell.zsh;
  });
}

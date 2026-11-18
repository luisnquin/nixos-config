{pkgs, ...}: {
  home.packages = [pkgs.winnow];

  programs.zsh.initContent = builtins.readFile (builtins.path {
    name = "agents-shrc";
    path = ./shell.zsh;
  });
}

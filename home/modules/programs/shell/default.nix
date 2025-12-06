{
  imports = [
    ./man.nix
    ./nao.nix
    ./translate.nix
    ./tty.nix
  ];

  shared = {
    bat.enable = true;
    btop.enable = true;
    direnv.enable = true;
    fzf.enable = true;
    macchina.enable = true;
    starship.enable = true;
    tmux.enable = true;
    zsh.enable = true;
  };

  programs.zsh.completionInit = builtins.readFile ../../.../../../../tools/nyx/completions.zsh;
}

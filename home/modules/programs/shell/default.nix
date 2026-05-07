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
    eza.enable = true;
    fzf.enable = true;
    less.enable = true;
    macchina.enable = true;
    magic-wormhole.enable = true;
    starship.enable = true;
    tmux = {
      enable = true;
      status = {
        ssh.enable = true;
        gpg.enable = true;
        lsyncd = {
          enable = true;
          hideOnRemoteSsh = true;
        };
        gitmux.enable = true;
      };
    };
    zoxide.enable = true;
    zsh.enable = true;
  };
}

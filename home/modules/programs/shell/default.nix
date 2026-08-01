{host, ...}: {
  imports = [
    ./man.nix
    ./nao.nix
    ./translate.nix
    ./tty.nix
  ];

  programs.zsh.initContent = builtins.readFile ./.zshrc;

  shared = {
    bat.enable = true;
    btop.enable = true;
    direnv.enable = true;
    eza.enable = true;
    fzf.enable = true;
    less.enable = true;
    macchina = {
      enable = true;
      ascii = host.banner;
    };
    magic-wormhole.enable = true;
    starship.enable = true;
    tmux = {
      enable = true;
      # Replaced by the attach-or-create launcher in programs/terminal.
      autoStart = false;
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

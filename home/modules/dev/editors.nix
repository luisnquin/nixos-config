{pkgs, ...}: {
  home = {
    packages = with pkgs; [
      neovim
      # vscode
      kibi
      vim
    ];

    file = {
      ".nanorc".text = ''
        include "${pkgs.nano}/share/nano/*.nanorc"
        include "${pkgs.nano}/share/nano/extra/*.nanorc"

        set titlecolor white,magenta
        set positionlog
        set autoindent
        set tabsize 4
        set atblanks
        set zero
      '';
    };
  };
}

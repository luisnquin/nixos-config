{
  config,
  pkgs,
  ...
}: {
  environment.systemPackages = with pkgs; [
    neovim
    vscode
    vim
  ];

  programs = {
    nano = {
      nanorc = ''
        set titlecolor white,magenta
        set positionlog
        set autoindent
        set tabsize 4
        set atblanks
        set zero
      '';

      syntaxHighlight = true;
    };
  };
}

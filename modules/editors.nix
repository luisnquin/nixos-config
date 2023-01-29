{
  config,
  pkgs,
  ...
}: let
  owner = import "/etc/nixos/owner.nix";
in {
  home-manager.users."${owner.username}" = with pkgs; {
    programs.vscode = {
      enable = true;
      enableExtensionUpdateCheck = true;
      enableUpdateCheck = true;
      package = vscode;
      extensions = with vscode-extensions; [
        matklad.rust-analyzer
        jnoortheen.nix-ide
      ];
      mutableExtensionsDir = true;
    };
  };

  environment.systemPackages = with pkgs; [
    neovim
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

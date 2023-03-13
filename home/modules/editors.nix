{
  config,
  pkgs,
  ...
}: {
  programs.vscode = {
    enable = true;
    enableExtensionUpdateCheck = true;
    enableUpdateCheck = true;
    package = pkgs.vscode;
    # extensions = with vscode-extensions; [
    #   matklad.rust-analyzer
    #   jnoortheen.nix-ide
    # ];
    mutableExtensionsDir = true;
  };
}

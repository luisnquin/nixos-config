{
  imports = let
    hmFork = builtins.fetchTarball {
      url = "https://github.com/sei40kr/home-manager/archive/vscode-fork-modules.tar.gz";
      sha256 = "0s43cs3883w12glndkfjsqrsy8hc05hg8yl92hvd2c2kxpmyqlih";
    };
  in [
    (import "${hmFork}/modules/programs/cursor.nix")
    (import "${hmFork}/modules/programs/antigravity.nix")
    ./antigravity.nix
    ./cursor.nix
  ];
}

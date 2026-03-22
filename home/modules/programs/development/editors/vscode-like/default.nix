{
  imports = let
    hmFork = builtins.fetchTarball {
      url = "https://github.com/sei40kr/home-manager/archive/vscode-fork-modules.tar.gz";
      sha256 = "1bq3i0rpiajvdq7n4ai05249rmi9k9cdx59m261p77hrr3jyaqr1";
    };
  in [
    (import "${hmFork}/modules/programs/cursor.nix")
    (import "${hmFork}/modules/programs/antigravity.nix")
    ./antigravity.nix
    ./cursor.nix
  ];
}

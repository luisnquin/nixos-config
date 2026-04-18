{
  imports = let
    hmFork = builtins.fetchTarball {
      url = "https://github.com/sei40kr/home-manager/archive/vscode-fork-modules.tar.gz";
      sha256 = "0jrd01ydyqq4w1qflzyw21a0n9qbgafy9ya6q4v02ygdcqhl92yz";
    };
  in [
    (import "${hmFork}/modules/programs/cursor.nix")
    (import "${hmFork}/modules/programs/antigravity.nix")
    ./antigravity.nix
    ./cursor.nix
  ];
}

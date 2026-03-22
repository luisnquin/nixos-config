{
  imports = let
    hmFork = builtins.fetchTarball {
      url = "https://github.com/sei40kr/home-manager/archive/vscode-fork-modules.tar.gz";
      sha256 = "0r5pv246jp1xc8g1kpzb0dyimpih9xrj7np6sabwmdwnc6pm3wyz";
    };
  in [
    (import "${hmFork}/modules/programs/cursor.nix")
    (import "${hmFork}/modules/programs/antigravity.nix")
    ./antigravity.nix
    ./cursor.nix
  ];
}

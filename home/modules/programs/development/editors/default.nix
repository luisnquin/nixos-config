{pkgs, ...}: {
  imports = [
    ./vscode-like.nix
    ./nano.nix
  ];

  home.packages = [pkgs.antigravity];
}

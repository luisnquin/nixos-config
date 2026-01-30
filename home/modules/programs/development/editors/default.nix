{pkgs, ...}: {
  imports = [
    ./vscode-like.nix
    ./nano.nix
    ./zed.nix
  ];

  home.packages = [pkgs.antigravity];
}

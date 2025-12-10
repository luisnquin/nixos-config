{pkgs, ...}: {
  imports = [
    ./vscode.nix
    ./nano.nix
    ./zed.nix
  ];

  home.packages = [pkgs.antigravity];
}

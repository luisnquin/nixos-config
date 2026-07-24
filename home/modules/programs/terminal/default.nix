{
  imports = [
    ./ghostty.nix
    ./herdr.nix
    ./ssh-gateway.nix
    ./tmux.nix
  ];

  shared.alacritty.enable = true;
}

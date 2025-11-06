{
  imports = [
    ./alacritty.nix
    ./ghostty.nix
  ];

  home.sessionPath = [
    "$HOME/.local/bin"
  ];
}

{
  imports = [
    ./ghostty.nix
    ./herdr.nix
    ./ssh-gateway.nix
  ];

  shared.alacritty.enable = true;

  programs.tmux.extraConfig = ''
    set -g allow-passthrough on
  '';
}

{
  config,
  pkgs,
  ...
}: {
  imports = [
    ./alacritty.nix
    ./zsh.nix
  ];
}

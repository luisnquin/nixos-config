{
  config,
  username,
  ...
}: {
  imports = [
    "/etc/nixos/modules/environment.nix"
    "/etc/nixos/modules/spotifyd.nix"
    "/etc/nixos/modules/nvidia.nix"
    "/etc/nixos/modules/tmux.nix"
    "/etc/nixos/modules/rust.nix"
    "/etc/nixos/modules/git.nix"
  ];
}

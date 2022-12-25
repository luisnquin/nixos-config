{config, ...}: {
  imports = [
    "/etc/nixos/modules/environment.nix"
    "/etc/nixos/modules/spotify.nix"
    "/etc/nixos/modules/nvidia.nix"
    "/etc/nixos/modules/tmux.nix"
    "/etc/nixos/modules/git.nix"
  ];
}

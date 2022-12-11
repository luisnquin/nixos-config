{config, ...}: {
  imports = [
    "/etc/nixos/modules/environment.nix"
    "/etc/nixos/modules/nvidia.nix"
    "/etc/nixos/modules/tmux.nix"
  ];
}

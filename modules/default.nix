{config, ...}: {
  imports = [
    "/etc/nixos/modules/virtual_host.nix"
    "/etc/nixos/modules/environment.nix"
    "/etc/nixos/modules/spotify.nix"
    "/etc/nixos/modules/docker.nix"
    "/etc/nixos/modules/nvidia.nix"
    "/etc/nixos/modules/tmux.nix"
    "/etc/nixos/modules/git.nix"
  ];
}

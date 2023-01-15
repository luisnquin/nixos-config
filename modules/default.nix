{config, ...}: {
  imports = [
    "/etc/nixos/modules/units/default.nix"
    "/etc/nixos/modules/virtual_host.nix"
    "/etc/nixos/modules/environment.nix"
    "/etc/nixos/modules/spotify.nix"
    "/etc/nixos/modules/desktop.nix"
    "/etc/nixos/modules/torrent.nix"
    "/etc/nixos/modules/docker.nix"
    "/etc/nixos/modules/nvidia.nix"
    "/etc/nixos/modules/shell.nix"
    "/etc/nixos/modules/tmux.nix"
    "/etc/nixos/modules/git.nix"
    "/etc/nixos/modules/nix.nix"
    "/etc/nixos/modules/k8s.nix"
  ];
}

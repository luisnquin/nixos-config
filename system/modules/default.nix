{config, ...}: {
  imports = [
    ./terminal
    ./dev

    ./virtual-machine.nix
    ./environment.nix
    ./security.nix
    ./desktop.nix
    ./torrent.nix
    ./docker.nix
    ./nvidia.nix
    ./fonts.nix
    ./nix.nix
    ./k8s.nix
  ];
}

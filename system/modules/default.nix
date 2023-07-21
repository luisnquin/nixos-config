{config, ...}: {
  imports = [
    ./terminal/default.nix
    ./virtual-machine.nix
    ./environment.nix
    ./security.nix
    # ./postgres.nix
    ./desktop.nix
    # ./torrent.nix
    ./docker.nix
    ./nvidia.nix
    # ./latex.nix
    ./dev
    ./fonts.nix
    ./nix.nix
    ./k8s.nix
  ];
}

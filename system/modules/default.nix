{config, ...}: {
  imports = [
    ./terminal/default.nix
    ./virtual_host.nix
    ./environment.nix
    ./security.nix
    ./desktop.nix
    ./torrent.nix
    ./editors.nix
    ./docker.nix
    ./nvidia.nix
    ./fonts.nix
    ./git.nix
    ./nix.nix
    ./k8s.nix
  ];
}
{config, ...}: {
  imports = [
    ./terminal/default.nix
    ./virtual-host.nix
    ./environment.nix
    ./security.nix
    ./desktop.nix
    ./torrent.nix
    ./editors.nix
    ./docker.nix
    ./nvidia.nix
    ./latex.nix
    ./fonts.nix
    ./git.nix
    ./nix.nix
    ./k8s.nix
    ./go.nix
  ];
}

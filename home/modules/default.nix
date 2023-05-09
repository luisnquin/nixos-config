{config, ...}: {
  imports = [
    ./terminal/default.nix
    ./editors.nix
    ./spotify.nix
    ./docker.nix
    ./btop.nix
    ./k8s.nix
    ./git.nix
  ];
}

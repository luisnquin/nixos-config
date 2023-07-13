{config, ...}: {
  imports = [
    ./terminal/default.nix
    ./macchina.nix
    ./editors.nix
    ./spotify.nix
    ./docker.nix
    ./btop.nix
    ./nao.nix
    ./k8s.nix
    ./git.nix
  ];
}

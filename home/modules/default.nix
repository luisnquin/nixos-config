{
  config,
  pkgs,
  ...
}: {
  imports = [
    ./terminal/default.nix
    ./editors.nix
    ./spotify.nix
    ./k8s.nix
  ];
}

{
  config,
  pkgs,
  ...
}: {
  imports = [
    ./terminal/defaut.nix
    ./editors.nix
    ./spotify.nix
    ./k8s.nix
  ];
}

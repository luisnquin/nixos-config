{...}: {
  imports = [
    ./terminal
    ./desktop
    ./dev

    # https://github.com/NixOS/nixpkgs/pull/249369
    # https://github.com/NixOS/nixpkgs/issues/249138
    # ./virtual-machine.nix
    ./environment.nix
    ./clipboard.nix
    ./security.nix
    ./torrent.nix
    ./docker.nix
    ./nvidia.nix
    ./fonts.nix
    ./boot.nix
    ./nix.nix
    ./k8s.nix
  ];
}

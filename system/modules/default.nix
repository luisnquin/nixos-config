{
  imports = [
    ./databases
    ./terminal
    ./security
    ./desktop
    ./network
    ./boot
    ./vm

    # https://github.com/NixOS/nixpkgs/pull/249369
    # https://github.com/NixOS/nixpkgs/issues/249138
    ./environment.nix
    ./bluetooth.nix
    ./clipboard.nix
    ./battery.nix
    ./docker.nix
    ./thunar.nix
    ./nvidia.nix
    ./fonts.nix
    ./audio.nix
    ./nix.nix
  ];
}

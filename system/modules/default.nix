{
  imports = [
    ./databases
    ./terminal
    ./desktop
    ./network
    ./battery
    ./boot
    ./vm

    # https://github.com/NixOS/nixpkgs/pull/249369
    # https://github.com/NixOS/nixpkgs/issues/249138
    ./environment.nix
    ./bluetooth.nix
    ./clipboard.nix
    ./security.nix
    ./docker.nix
    ./thunar.nix
    ./nvidia.nix
    ./fonts.nix
    ./audio.nix
    ./vpn.nix
    ./nix.nix
  ];
}

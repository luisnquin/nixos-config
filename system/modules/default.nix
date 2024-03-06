{
  imports = [
    ./databases
    ./terminal
    ./desktop
    ./network
    ./battery
    ./boot

    # https://github.com/NixOS/nixpkgs/pull/249369
    # https://github.com/NixOS/nixpkgs/issues/249138
    ./virtual-machine.nix
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

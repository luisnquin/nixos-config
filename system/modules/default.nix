{
  imports = [
    ./terminal
    ./desktop
    ./dev

    # https://github.com/NixOS/nixpkgs/pull/249369
    # https://github.com/NixOS/nixpkgs/issues/249138
    ./virtual-machine.nix
    ./environment.nix
    ./bluetooth.nix
    ./clipboard.nix
    ./security.nix
    ./battery.nix
    ./network.nix
    ./docker.nix
    ./thunar.nix
    ./nvidia.nix
    ./fonts.nix
    ./audio.nix
    ./boot.nix
    ./vpn.nix
    ./nix.nix
  ];
}

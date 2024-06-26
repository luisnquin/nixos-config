{
  imports = [
    ./databases
    ./terminal
    ./security
    ./desktop
    ./network
    ./boot
    # ./vm

    # https://github.com/NixOS/nixpkgs/pull/249369
    # https://github.com/NixOS/nixpkgs/issues/249138
    ./environment.nix
    ./bluetooth.nix
    ./clipboard.nix
    ./graphics.nix
    ./battery.nix
    ./systemd.nix
    ./docker.nix
    ./thunar.nix
    ./fonts.nix
    ./audio.nix
    ./dbus.nix
    ./nix.nix
  ];
}

{
  imports = [
    ./terminal
    ./security
    ./desktop
    ./network
    ./boot
    ./fs
    ./vm

    # https://github.com/NixOS/nixpkgs/pull/249369
    # https://github.com/NixOS/nixpkgs/issues/249138
    ./essentials.nix
    ./bloatware.nix
    ./bluetooth.nix
    ./clipboard.nix
    ./graphics.nix
    ./printing.nix
    ./battery.nix
    ./flatpak.nix
    ./android.nix
    ./systemd.nix
    ./kernel.nix
    ./docker.nix
    ./gaming.nix
    ./thunar.nix
    ./fonts.nix
    ./audio.nix
    ./dbus.nix
    ./nix.nix
  ];
}

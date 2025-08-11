{
  imports = [
    ./compat.nix
    ./dbus.nix
    ./kernel.nix
    ./systemd.nix
    ./tools.nix
  ];

  environment.localBinInPath = true;
}

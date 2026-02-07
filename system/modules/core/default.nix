{
  imports = [
    ./compat.nix
    ./dbus.nix
    ./kernel.nix
    ./systemd-journald.nix
    ./tools.nix
  ];

  environment.localBinInPath = true;
}

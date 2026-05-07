{
  imports = [
    ./compat.nix
    ./dbus.nix
    ./kernel.nix
    ./systemd-journald.nix
    ./tools.nix
  ];

  environment = {
    localBinInPath = true;
    profiles = [
      "$HOME/.nix-profile"
      "/etc/profiles/per-user/$USER"
      "/nix/var/nix/profiles/default"
      "/run/current-system/sw"
    ];
  };
}

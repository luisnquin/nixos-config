{lib, ...}: {
  imports = [
    ./compat.nix
    ./dbus.nix
    ./kernel.nix
    ./systemd-journald.nix
    ./tools.nix
  ];

  environment = {
    localBinInPath = true;
    profiles = lib.mkForce [
      "$HOME/.local/state/nix/profiles/profile"
      "/etc/profiles/per-user/$USER"
      "/nix/var/nix/profiles/default"
      "/run/current-system/sw"
      "$HOME/.local/share/flatpak/exports"
      "/var/lib/flatpak/exports"
    ];
  };
}

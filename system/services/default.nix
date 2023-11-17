{...}: {
  imports = [
    ./successful-ping-to-google.nix
    ./discord-skip-host-update.nix
    ./battery-threshold.nix
    ./battery-notifier.nix
    ./clean-trash-bin.nix
    ./ensure-home-fs.nix
    ./tldr-update.nix
  ];
}

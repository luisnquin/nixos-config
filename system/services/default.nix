{config, ...}: {
  imports = [
    ./successful-ping-to-google.nix
    ./discord-skip-host-update.nix
    ./clean-trash-bin.nix
    ./ensure-home-fs.nix
    ./battery-limit.nix
    ./tldr-update.nix
  ];
}

{config, ...}: {
  imports = [
    ./successful-ping-to-google.nix
    ./discord-skip-host-update.nix
    ./ensure-home-fs.nix
    ./battery-limit.nix
    ./tldr-update.nix
  ];
}

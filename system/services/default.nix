{config, ...}: {
  imports = [
    ./successful-ping-to-google.nix
    ./ensure-home-fs.nix
    ./battery-limit.nix
    ./tldr-update.nix
  ];
}

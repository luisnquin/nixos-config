{config, ...}: {
  imports = [
    ./successful-ping-to-google.nix
    ./battery-limit.nix
    ./tldr-update.nix
  ];
}

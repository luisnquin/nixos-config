{config, ...}: {
  imports = [
    ./successful-ping-to-google.nix
    ./battery-limit.nix
  ];
}

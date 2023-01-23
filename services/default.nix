{config, ...}: {
  imports = [
    "/etc/nixos/services/successful-ping-to-google.nix"
    "/etc/nixos/services/environment-info.nix"
    "/etc/nixos/services/battery-limit.nix"
  ];
}

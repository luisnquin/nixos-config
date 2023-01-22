{config, ...}: {
  imports = [
    "/etc/nixos/modules/services/successful-ping-to-google.nix"
    "/etc/nixos/modules/services/environment-info.nix"
    "/etc/nixos/modules/services/battery-limit.nix"
  ];
}

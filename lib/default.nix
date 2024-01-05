{
  pkgs,
  lib,
}:
builtins.mapAttrs (_name: path: import path {inherit pkgs lib;}) {
  formats = ./formats.nix;
  base64 = ./base64.nix;
  fs = ./fs.nix;
}
// import ./metadata.nix
// import ./machine.nix

{
  pkgs,
  lib,
}:
builtins.mapAttrs (name: path: import path {inherit pkgs lib;}) {
  base64 = ./base64.nix;
  fs = ./fs.nix;
}
// import ./metadata.nix
// import ./machine.nix
